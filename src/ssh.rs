use std::{path, process};
use errors::*;
use ::{Device, PlatformManager};

use config::SshDeviceConfiguration;

#[derive(Clone,Debug)]
pub struct SshDevice {
    id: String,
    config: SshDeviceConfiguration,
}

impl Device for SshDevice {
    fn name(&self) -> &str {
        &*self.id
    }
    fn id(&self) -> &str {
        &*self.id
    }
    fn target(&self) -> String {
        "armv7-unknown-linux-gnueabihf".to_string()
    }
    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
    }
    fn make_app(&self, exe: &path::Path) -> Result<path::PathBuf> {
        ::make_linux_app(exe)
    }
    fn install_app(&self, app: &path::Path) -> Result<()> {
        let user_at_host = format!("{}@{}", self.config.username, self.config.hostname);
        let _stat = process::Command::new("ssh").args(&[&*user_at_host, "rm", "-rf", "/tmp/dinghy"])
            .status()?;
        let stat = process::Command::new("ssh").args(&[&*user_at_host, "mkdir", "/tmp/dinghy"])
            .status()?;
        if !stat.success() {
            Err("error creating /tmp/dinghy")?
        }
        let target_path = format!("/tmp/dinghy/{:?}", app.file_name().unwrap());
        let stat = process::Command::new("rsync").arg("-a")
            .arg(&*format!("{}/", app.to_str().unwrap()))
            .arg(&*format!("{}:{}/", user_at_host, &*target_path))
            .status()?;
        if !stat.success() {
            Err("error installing app")?
        }
        Ok(())
    }
    fn run_app(&self, app_path: &path::Path, args: &[&str]) -> Result<()> {
        let user_at_host = format!("{}@{}", self.config.username, self.config.hostname);
        let app_name = app_path.file_name().unwrap();
        let path = path::PathBuf::from("/tmp/dinghy").join(app_name);
        let exe = path.join(&app_name);
        let stat = process::Command::new("ssh").arg(user_at_host)
            .arg(&*format!("DINGHY=1 {}", &exe.to_str().unwrap()))
            .args(args)
            .status()?;
        if !stat.success() {
            Err("test fail.")?
        }
        Ok(())
    }
    fn debug_app(&self, _app_path: &path::Path, _args: &[&str]) -> Result<()> {
        unimplemented!()
    }
}

pub struct SshDeviceManager {
}

impl SshDeviceManager {
    pub fn probe() -> Option<SshDeviceManager> {
        Some(SshDeviceManager {})
    }
}

impl PlatformManager for SshDeviceManager {
    fn devices(&self) -> Result<Vec<Box<Device>>> {
        Ok(::config::config()?
            .ssh_devices
            .iter()
            .map(|(k, d)| {
                Box::new(SshDevice {
                    id: k.clone(),
                    config: d.clone(),
                }) as _
            })
            .collect())
    }
}
