use std::{path, process};
use errors::*;
use {Device, PlatformManager};

use config::SshDeviceConfiguration;

#[derive(Clone, Debug)]
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
        self.config.target.to_string()
    }
    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
    }
    fn make_app(&self, source: &path::Path, exe: &path::Path) -> Result<path::PathBuf> {
        ::make_linux_app(source, exe)
    }
    fn install_app(&self, app: &path::Path) -> Result<()> {
        let user_at_host = format!("{}@{}", self.config.username, self.config.hostname);
        let prefix = self.config.path.clone().unwrap_or("/tmp".into());
        let _stat = process::Command::new("ssh")
            .args(
                &[
                    &*user_at_host,
                    "-p",
                    &*format!("{}", self.config.port.unwrap_or(22)),
                    "mkdir",
                    "-p",
                    &*format!("{}/dinghy", prefix),
                ],
            )
            .status();
        let target_path = format!(
            "{}/dinghy/{}",
            prefix,
            app.file_name().unwrap().to_str().unwrap()
        );
        info!("Rsyncing to {}", self.name());
        println!(
            "{}/ {}:{}/",
            app.to_str().unwrap(),
            user_at_host,
            &*target_path
        );
        let stat = process::Command::new("/usr/bin/rsync")
            .arg("-a")
            .arg("-v")
            .arg("-e")
            .arg(&*format!("ssh -p {}", self.config.port.unwrap_or(22)))
            .arg(&*format!("{}/", app.to_str().unwrap()))
            .arg(&*format!("{}:{}/", user_at_host, &*target_path))
            .status()?;
        if !stat.success() {
            Err("error installing app")?
        }
        Ok(())
    }
    fn clean_app(&self, app_path: &path::Path) -> Result<()> {
        let user_at_host = format!("{}@{}", self.config.username, self.config.hostname);
        let prefix = self.config.path.clone().unwrap_or("/tmp".into());
        let app_name = app_path.file_name().unwrap();
        let path = path::PathBuf::from(prefix)
            .join("dinghy")
            .join(app_name);
        let stat = process::Command::new("ssh")
            .arg(user_at_host)
            .arg("-p")
            .arg(&*format!("{}", self.config.port.unwrap_or(22)))
            .arg(&*format!(
                "rm -rf {}",
                &path.to_str().unwrap()
            ))
            .status()?;
        if !stat.success() {
            Err("test fail.")?
        }
        Ok(())
    }
    fn run_app(&self, app_path: &path::Path, args: &[&str], envs: &[&str]) -> Result<()> {
        let user_at_host = format!("{}@{}", self.config.username, self.config.hostname);
        let prefix = self.config.path.clone().unwrap_or("/tmp".into());
        let app_name = app_path.file_name().unwrap();
        let path = path::PathBuf::from(prefix)
            .join("dinghy")
            .join(app_name);
        let exe = path.join(&app_name);
        let stat = process::Command::new("ssh")
            .arg(user_at_host)
            .arg("-p")
            .arg(&*format!("{}", self.config.port.unwrap_or(22)))
            .arg(&*format!(
                "DINGHY=1 {} {}",
                envs.join(" "),
                &exe.to_str().unwrap()
            ))
            .args(args)
            .status()?;
        if !stat.success() {
            Err("test fail.")?
        }
        Ok(())
    }
    fn debug_app(&self, _app_path: &path::Path, _args: &[&str], _envs: &[&str]) -> Result<()> {
        unimplemented!()
    }
}

pub struct SshDeviceManager {}

impl SshDeviceManager {
    pub fn probe() -> Option<SshDeviceManager> {
        Some(SshDeviceManager {})
    }
}

impl PlatformManager for SshDeviceManager {
    fn devices(&self) -> Result<Vec<Box<Device>>> {
        Ok(
            ::config::config(::std::env::current_dir().unwrap())?
                .ssh_devices
                .iter()
                .map(|(k, d)| {
                    Box::new(SshDevice {
                        id: k.clone(),
                        config: d.clone(),
                    }) as _
                })
                .collect(),
        )
    }
}
