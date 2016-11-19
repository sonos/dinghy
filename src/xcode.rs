use std::{env, fs, io, path, process};
use std::io::Write;

use errors::*;

pub fn wrap_as_app<P: AsRef<path::Path>>(target: &str,
                                         mode: &str,
                                         name: &str,
                                         executable: P,
                                         app_bundle_id:&str)
                                         -> Result<path::PathBuf> {
    let app_name = app_bundle_id.split(".").last().unwrap();
    let app_path = path::Path::new("target")
        .join(target)
        .join(mode)
        .join("dinghy")
        .join(name)
        .join(format!("{}.app", app_name));
    let _ = fs::remove_dir_all(&app_path);
    fs::create_dir_all(&app_path)?;
    fs::copy(&executable, app_path.join(app_name))?;
    let mut plist = fs::File::create(app_path.join("Info.plist"))?;
    writeln!(plist, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    writeln!(plist, r#"<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">"#)?;
    writeln!(plist, r#"<plist version="1.0"><dict>"#)?;
    writeln!(plist,
             "<key>CFBundleExecutable</key><string>{}</string>", app_name)?;
    writeln!(plist,
             "<key>CFBundleIdentifier</key><string>{}</string>", app_bundle_id)?;
    writeln!(plist, "<key>UIRequiredDeviceCapabilities</key>")?;
    writeln!(plist,
             "<array><string>{}</string></array>",
             target.split("-").next().unwrap())?;
    writeln!(plist, r#"</dict></plist>"#)?;
    Ok(app_path)
}

pub fn sign_app<P: AsRef<path::Path>>(app: P, settings:&SignatureSettings) -> Result<()> {
    info!("Will sign {:?} with team: {} using key: {}",
          app.as_ref(),
          settings.identity.team,
          settings.identity.name);

    let entitlements =
        app.as_ref().parent().ok_or("not building in root")?.join("entitlements.xcent");
    debug!("entitlements file: {}", entitlements.to_str().unwrap_or(""));
    let mut plist = fs::File::create(&entitlements)?;
    writeln!(plist, r#"<?xml version="1.0" encoding="UTF-8"?>"#)?;
    writeln!(plist, r#"<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">"#)?;
    writeln!(plist, r#"<plist version="1.0"><dict>"#)?;
    writeln!(plist, "{}", settings.entitlements)?;
    writeln!(plist, r#"</dict></plist>"#)?;

    process::Command::new("codesign").args(&[
                "-s", &*settings.identity.name,
                "--entitlements",
                entitlements.to_str().ok_or("not utf8 path")?,
                app.as_ref().to_str().ok_or("not utf8 path")?])
        .status()?;
    Ok(())
}

#[derive(Debug,Clone)]
pub struct SignatureSettings {
    pub identity: SigningIdentity,
    pub entitlements: String,
    pub name: String,
    pub profile: String,
}

#[derive(Debug,Clone)]
pub struct SigningIdentity {
    pub id: String,
    pub name: String,
    pub team: String,
}

pub fn look_for_signature_settings(device_id: &str) -> Result<Vec<SignatureSettings>> {
    let identity_regex = ::regex::Regex::new(r#"^ *[0-9]+\) ([A-Z0-9]{40}) "(.+)"$"#)?;
    let subject_regex = ::regex::Regex::new(r#"OU=([^,]+)"#)?;
    let mut identities: Vec<SigningIdentity> = vec![];
    let find_identities =
        process::Command::new("security").args(&["find-identity", "-v", "-p", "codesigning"])
            .output()?;
    for line in String::from_utf8(find_identities.stdout)?.split("\n") {
        if let Some(caps) = identity_regex.captures(&line) {
            let name: String = caps[2].into();
            if !name.starts_with("iPhone Developer: ") {
                continue;
            }
            let subject = process::Command::new("sh").arg("-c")
                .arg(format!("security find-certificate -a -c \"{}\" -p | openssl x509 -text | \
                              grep Subject:",
                             name))
                .output()?;
            let subject = String::from_utf8(subject.stdout)?;
            if let Some(ou) = subject_regex.captures(&subject) {
                identities.push(SigningIdentity {
                    id: caps[1].into(),
                    name: caps[2].into(),
                    team: ou[1].into(),
                })
            }
        }
    }
    debug!("signing identities: {:?}", identities);
    let mut settings = vec!();
    for file in fs::read_dir(env::home_dir()
        .unwrap()
        .join("Library/MobileDevice/Provisioning Profiles"))? {
        let file = file?;
        debug!("considering profile {:?}", file.path());
        let decoded = process::Command::new("security").arg("cms")
            .arg("-D")
            .arg("-i")
            .arg(file.path())
            .output()?;
        let plist = ::plist::Plist::read(io::Cursor::new(&decoded.stdout))?;
        let dict = plist.as_dictionary().ok_or("plist root should be a dictionary")?;
        let devices = if let Some(d) = dict.get("ProvisionedDevices") {
            d
        } else {
            debug!("  no devices in profile");
            continue;
        };
        let devices = if let Some(ds) = devices.as_array() {
            ds
        } else {
            Err("ProvisionedDevices expected to be array")?
        };
        if !devices.contains(&::plist::Plist::String(device_id.into())) {
            debug!("  no device match in profile");
            continue;
        }
        let name = dict.get("Name").ok_or(format!("No name in profile {:?}", file.path()))?;
        let name = name.as_string()
            .ok_or(format!("Name should have been a string in {:?}", file.path()))?;
        if !name.ends_with("Dinghy") {
            continue;
        }
        // TODO: check date in future
        let team = dict.get("TeamIdentifier").ok_or("no TeamIdentifier")?;
        let team = team.as_array().ok_or("TeamIdentifier should be an array")?;
        let team = team.first()
            .ok_or("empty TeamIdentifier")?
            .as_string()
            .ok_or("TeamIdentifier should be a String")?
            .to_string();
        let identity = identities.iter().find(|i| i.team == team);
        if identity.is_none() {
            debug!("no identity for team");
            continue;
        }
        let identity = identity.unwrap();
        let entitlements = String::from_utf8(decoded.stdout)?
            .split("\n")
            .skip_while(|line| !line.contains("<key>Entitlements</key>"))
            .skip(2)
            .take_while(|line| !line.contains("</dict>"))
            .collect::<Vec<&str>>()
            .join("\n");
        settings.push(SignatureSettings {
            entitlements: entitlements,
            name: name.into(),
            identity: identity.clone(),
            profile: file.path().to_str().unwrap().into(),
        });
    }
    Ok(settings)
}
