pub use self::device::{IosDevice, IosSimDevice};
pub use self::platform::IosPlatform;
use crate::{Device, Platform, PlatformManager, Result};

mod device;
mod platform;
mod xcode;

use anyhow::{anyhow, Context};
use log::info;

#[derive(Debug, Clone)]
pub struct SignatureSettings {
    pub identity: SigningIdentity,
    pub file: String,
    pub entitlements: String,
    pub name: String,
    pub profile: String,
}

#[derive(Debug, Clone)]
pub struct SigningIdentity {
    pub id: String,
    pub name: String,
    pub team: String,
}

pub struct IosManager {
    devices: Vec<Box<dyn Device>>,
}

impl IosManager {
    pub fn new() -> Result<Option<IosManager>> {
        let devices = devices()
            .context("Could not list iOS devices")?
            .into_iter()
            .chain(
                simulators()
                    .context("Could not list iOS simulators")?
                    .into_iter(),
            )
            .collect();
        Ok(Some(IosManager { devices }))
    }
}

impl PlatformManager for IosManager {
    fn devices(&self) -> Result<Vec<Box<dyn Device>>> {
        Ok(self.devices.clone())
    }

    fn platforms(&self) -> Result<Vec<Box<dyn Platform>>> {
        [
            "armv7",
            "armv7s",
            "aarch64",
            "i386",
            "x86_64",
            "aarch64-sim",
        ]
        .iter()
        .map(|arch| {
            let id = format!("auto-ios-{}", arch);
            let rustc_triple = if *arch != "aarch64-sim" {
                format!("{}-apple-ios", arch)
            } else {
                format!("aarch64-apple-ios-sim")
            };
            IosPlatform::new(
                id,
                &rustc_triple,
                crate::config::PlatformConfiguration::default(),
            )
            .map(|pf| pf as Box<dyn Platform>)
        })
        .collect()
    }
}

fn simulators() -> Result<Vec<Box<dyn Device>>> {
    let sims_list = ::std::process::Command::new("xcrun")
        .args(&["simctl", "list", "--json", "devices"])
        .output()?;
    if !sims_list.status.success() {
        info!(
                "Failed while looking for ios simulators. It this is not expected, you need to make sure `xcrun simctl list --json` works."
            );
        return Ok(vec![]);
    }
    let sims_list = String::from_utf8(sims_list.stdout)?;
    let sims_list = json::parse(&sims_list)
               .with_context(|| "Could not parse output for: `xcrun simctl list --json devices` as json. Please try to make this command work and retry.")?;
    let mut sims: Vec<Box<dyn Device>> = vec![];
    for (ref k, ref v) in sims_list["devices"].entries() {
        for ref sim in v.members() {
            if sim["state"] == "Booted" {
                sims.push(Box::new(IosSimDevice {
                    name: sim["name"]
                        .as_str()
                        .ok_or_else(|| anyhow!("unexpected simulator list format (missing name)"))?
                        .to_string(),
                    id: sim["udid"]
                        .as_str()
                        .ok_or_else(|| anyhow!("unexpected simulator list format (missing udid)"))?
                        .to_string(),
                    os: k.split(" ").last().unwrap().to_string(),
                }))
            }
        }
    }
    Ok(sims)
}

fn devices() -> Result<Vec<Box<dyn Device>>> {
    let list = ::std::process::Command::new("ios-deploy")
        .stderr(std::process::Stdio::inherit())
        .args(&["-c", "--json", "-t", "1"])
        .output();
    let list = match list {
        Ok(l) => l,
        Err(e) => {
            info!(
                "Could not execute ios-deploy to look for iOS devices ({}), so iOS device support is disabled. Consider installing ios-deploy (`brew install ios-deploy`...) for iOS support.", e);
            return Ok(vec![]);
        }
    };
    if !list.status.success() {
        info!(
                "ios-deploy returned an error while listing devices. It this is not expected, you need to make sure `ios-deploy --json -c -t 1` works as expected."
            );
        return Ok(vec![]);
    }
    // ios-deploy outputs each device as a multiline json dict, with separator or delimiter. make
    // it a json array.
    let list = String::from_utf8(list.stdout)?.replace("}{", "},{");
    let list = format!("[{}]", list);
    let list = ::json::parse(&list)
               .with_context(|| "Could not parse output for: `ios-deploy --json -c -t 1` as json. Please try to make this command work and retry.")?;
    list.members()
        .map(|json| Ok(Box::new(IosDevice::new(&json)?) as Box<dyn Device>))
        .collect::<Result<Vec<Box<dyn Device>>>>()
}
