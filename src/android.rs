use std::{env, path};
use std::process::{Command, Stdio};

use errors::*;
use Device;
use PlatformManager;
use DeviceCompatibility;
use project::Project;
use regular_platform::RegularPlatform;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug)]
pub struct AndroidDevice {
    adb: String,
    id: String,
    supported_targets: Vec<&'static str>,
}

impl AndroidDevice {
    fn from_id(adb: String, id: &str) -> Result<AndroidDevice> {
        let getprop_output = Command::new(&adb)
            .args(&["-s", id, "shell", "getprop", "ro.product.cpu.abilist"])
            .output()?;
        let abilist = String::from_utf8(getprop_output.stdout)?;
        let supported_targets = abilist
            .trim()
            .split(",")
            .filter_map(|abi| {
                Some(match abi {
                    "arm64-v8a" => "aarch64-linux-android",
                    "armeabi-v7a" => "armv7-linux-androideabi",
                    "armeabi" => "arm-linux-androideabi",
                    "x86" => "i686-linux-android",
                    _ => return None,
                })
            })
            .collect::<Vec<_>>();

        let device = AndroidDevice {
            adb,
            id: id.into(),
            supported_targets: supported_targets,
        };
        debug!("device: {:?}", device);
        Ok(device)
    }
}

impl DeviceCompatibility for AndroidDevice {
    fn is_compatible_with_regular_platform(&self, platform: &RegularPlatform) -> bool {
        self.supported_targets.contains(&platform.toolchain.tc_triple.as_str())
    }
}

impl Device for AndroidDevice {
    fn name(&self) -> &str {
        "android device"
    }
    fn id(&self) -> &str {
        &*self.id
    }
    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
    }
    fn make_app(&self, project: &Project, source: &path::Path, exe: &path::Path) -> Result<path::PathBuf> {
        use std::fs;
        let exe_file_name = exe.file_name()
            .expect("app should be a file in android mode");

        let bundle_path = exe.parent().ok_or("no parent")?.join("dinghy");
        let bundled_exe_path = bundle_path.join(exe_file_name);

        debug!("Removing previous bundle {:?}", bundle_path);
        let _ = fs::remove_dir_all(&bundle_path);

        debug!("Making bundle {:?} for {:?}", bundle_path, exe);
        fs::create_dir_all(&bundle_path)?;

        debug!("Copying exe to bundle");
        fs::copy(&exe, &bundled_exe_path)?;

        debug!("Copying src to bundle");
        project.rec_copy(source, &bundle_path, false)?;

        debug!("Copying test_data to bundle");
        project.copy_test_data(&bundle_path)?;

        Ok(bundled_exe_path.into())
    }
    fn install_app(&self, exe: &path::Path) -> Result<()> {
        let exe_name = exe.file_name()
            .and_then(|p| p.to_str())
            .expect("exe should be a file in android mode");
        let exe_parent = exe.parent()
            .and_then(|p| p.to_str())
            .expect("exe must have a parent");

        let target_dir = format!("/data/local/tmp/dinghy/{}", exe_name);
        let target_exec = format!("{}/{}", target_dir, exe_name);

        debug!("Clear existing files");
        let _stat = Command::new(&self.adb)
            .args(&["-s", &*self.id, "shell", "rm", "-rf", &*target_dir])
            .status()?;

        debug!("Push entire parent dir of exe");
        let stat = Command::new(&self.adb)
            .args(&["-s", &*self.id, "push", exe_parent, &*target_dir])
            .status()?;
        if !stat.success() {
            Err("failure in android install")?;
        }

        debug!("chmod target exe");
        let stat = Command::new(&self.adb)
            .args(&["-s", &*self.id, "shell", "chmod", "755", &*target_exec])
            .status()?;
        if !stat.success() {
            Err("failure in android install")?;
        }

        Ok(())
    }
    fn clean_app(&self, exe: &path::Path) -> Result<()> {
        let exe_name = exe.file_name()
            .and_then(|p| p.to_str())
            .expect("exe should be a file in android mode");

        let target_dir = format!("/data/local/tmp/dinghy/{}", exe_name);

        debug!("rm target exe");
        let stat = Command::new(&self.adb)
            .args(&["-s", &*self.id, "shell", "rm", "-rf", &*target_dir])
            .status()?;
        if !stat.success() {
            Err("failure in android clean")?;
        }

        Ok(())
    }
    fn run_app(&self, exe: &path::Path, args: &[&str], envs: &[&str]) -> Result<()> {
        let exe_name = exe.file_name()
            .and_then(|p| p.to_str())
            .expect("exe should be a file in android mode");

        let target_dir = format!("/data/local/tmp/dinghy/{}", exe_name);
        let target_exe = format!("{}/{}", target_dir, exe_name);

        let stat = Command::new(&self.adb)
            .arg("-s")
            .arg(&*self.id)
            .arg("shell")
            .arg(&*format!(
                "cd {:?}; DINGHY=1 {}",
                target_dir,
                envs.join(" ")
            ))
            .arg(&*target_exe)
            .args(args)
            .status()?;
        if !stat.success() {
            Err("failure in android run")?;
        }
        Ok(())
    }
    fn debug_app(&self, _app_path: &path::Path, _args: &[&str], _envs: &[&str]) -> Result<()> {
        unimplemented!()
    }
}

impl Display for AndroidDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Ok(fmt.write_str(format!("Android {{ \"id\": \"{}\", \"supported_targets\": {:?} }}",
                                 self.id,
                                 self.supported_targets).as_str())?)
    }
}

fn adb() -> Result<String> {
    fn try_out(command: &str) -> bool {
        match Command::new(command)
            .arg("--version")
            .stdout(Stdio::null())
            .status()
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }
    if try_out("fb_adb") {
        return Ok("fb-adb".into());
    }
    if try_out("adb") {
        return Ok("adb".into());
    }
    if let Ok(home) = env::var("HOME") {
        let mac_place = format!("{}/Library/Android/sdk/platform-tools/adb", home);
        if try_out(&mac_place) {
            return Ok(mac_place);
        }
    }
    Err("Neither fb-adb or adb could be found")?
}

pub struct AndroidManager {
    adb: String,
}

impl PlatformManager for AndroidManager {
    fn devices(&self) -> Result<Vec<Box<Device>>> {
        let result = Command::new(&self.adb).arg("devices").output()?;
        let mut devices = vec![];
        let device_regex = ::regex::Regex::new(r#"^(\S+)\tdevice\r?$"#)?;
        for line in String::from_utf8(result.stdout)?.split("\n").skip(1) {
            if let Some(caps) = device_regex.captures(line) {
                let d = AndroidDevice::from_id(self.adb.clone(), &caps[1])?;
                debug!("Discovered Android device {:?}", d);
                devices.push(Box::new(d) as Box<Device>);
            }
        }
        Ok(devices)
    }
}

impl AndroidManager {
    pub fn probe() -> Option<AndroidManager> {
        match adb() {
            Ok(adb) => {
                info!("Using {}", adb);
                Some(AndroidManager { adb })
            }
            Err(_) => {
                info!("adb not found in path, android disabled");
                None
            }
        }
    }
}
