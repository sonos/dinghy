use std::path;
use std::process::Command;

use errors::*;
use ::{Device, PlatformManager};

#[derive(Debug,Clone)]
pub struct AndroidDevice {
    id: String,
}


impl AndroidDevice {
    fn from_id(id: &str) -> Result<AndroidDevice> {
        let device = AndroidDevice { id: id.into() };
        Ok(device)
    }
}

impl Device for AndroidDevice {
    fn name(&self) -> &str {
        "i'm a droid"
    }
    fn id(&self) -> &str {
        &*self.id
    }
    fn target(&self) -> String {
        "arm-linux-androideabi".to_string()
    }
    fn can_run(&self, target:&str) -> bool {
        target.ends_with("-linux-androideabi")
    }
    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
    }
    fn make_app(&self, exe: &path::Path) -> Result<path::PathBuf> {
        ::make_linux_app(exe)
    }
    fn install_app(&self, app: &path::Path) -> Result<()> {
        let name = app.file_name().expect("app should be a file in android mode");
        let target_path = format!("/data/local/tmp/{}", name.to_str().unwrap_or("dinghy"));
        let _stat = Command::new("adb")
            .args(&["-s", &*self.id, "shell", "rm", "-rf", &*target_path])
            .status()?;
        let stat = Command::new("adb")
            .args(&["-s", &*self.id, "push", app.to_str().unwrap(), &*target_path])
            .status()?;
        if !stat.success() {
            Err("failure in android install")?;
        }
        // required when pushing from windows
        let stat = Command::new("adb")
            .args(&["-s", &*self.id, "shell", "chmod", "755", &*target_path])
            .status()?;
        if !stat.success() {
            Err("failure in android install")?;
        }
        Ok(())
    }
    fn run_app(&self, app_path: &path::Path, args: &[&str]) -> Result<()> {
        let name = app_path.file_name().expect("app should be a file in android mode");
        let target_name = format!("/data/local/tmp/{name}/{name}", name=name.to_str().unwrap_or("dinghy"));
        let stat = Command::new("adb").arg("-s")
            .arg(&*self.id)
            .arg("shell")
            .arg(&*target_name)
            .args(args)
            .status()?;
        // FIXME: consider switching to fb-adb to get error status
        if !stat.success() {
            Err("failure in android run")?;
        }
        Ok(())
    }
    fn debug_app(&self, _app_path: &path::Path, _args: &[&str]) -> Result<()> {
        unimplemented!()
    }
}

pub struct AndroidManager {
}

impl PlatformManager for AndroidManager {
    fn devices(&self) -> Result<Vec<Box<Device>>> {
        let result = Command::new("adb").arg("devices").output()?;
        let mut devices = vec![];
        let device_regex = ::regex::Regex::new(r#"^(\S+)\tdevice\r?$"#)?;
        for line in String::from_utf8(result.stdout)?.split("\n").skip(1) {
            if let Some(caps) = device_regex.captures(line) {
                let d = AndroidDevice::from_id(&caps[1])?;
                devices.push(Box::new(d) as Box<Device>);
            }
        }
        Ok(devices)
    }
}

impl AndroidManager {
    pub fn probe() -> Option<AndroidManager> {
        match Command::new("adb").arg("devices").output() {
            Ok(_) => {
                info!("adb found in path, android enabled");
                Some(AndroidManager {})
            }
            Err(_) => {
                info!("adb not found in path, android disabled");
                None
            }
        }

    }
}
