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
    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
    }
    fn make_app(&self, app: &path::Path) -> Result<path::PathBuf> {
        Ok(app.into())
    }
    fn install_app(&self, app: &path::Path) -> Result<()> {
        let name = app.file_name().expect("app should be a file in android mode");
        let target_name = format!("/data/local/tmp/{}", name.to_str().unwrap_or("dinghy"));
        Command::new("adb").arg("-s")
            .arg(&*self.id)
            .arg("push")
            .arg(app)
            .arg(&*target_name)
            .status()?;
        Command::new("adb").arg("-s")
            .arg(&*self.id)
            .arg("shell")
            .arg("chmod")
            .arg("775")
            .arg(&*target_name)
            .status()?;
        Ok(())
    }
    fn run_app(&self, app_path: &path::Path, args: &[&str]) -> Result<()> {
        let name = app_path.file_name().expect("app should be a file in android mode");
        let target_name = format!("/data/local/tmp/{}", name.to_str().unwrap_or("dinghy"));
        Command::new("adb").arg("-s")
            .arg(&*self.id)
            .arg("shell")
            .arg(&*target_name)
            .args(args)
            .status()?;
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
