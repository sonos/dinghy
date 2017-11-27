use std::{path, process};
use errors::*;
use {Device, PlatformManager,Toolchain};

use config::SshDeviceConfiguration;

#[derive(Clone, Debug)]
pub struct SshDevice {
    id: String,
    config: SshDeviceConfiguration,
}

impl SshDevice {
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
    fn toolchain(&self, _target: &str) -> Result<Box<Toolchain>> {
        let tc = self.config.toolchain.as_ref().ok_or("Ssh target with no default configuration")?;
        ::regular_toolchain::RegularToolchain::new(tc)
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
        let _stat = if let Some(port) = self.config.port {
            process::Command::new("ssh")
                .args(&[
                    &*user_at_host,
                    "-p",
                    &*format!("{}", port),
                    "mkdir",
                    "-p",
                    &*format!("{}/dinghy", prefix),
                ])
                .status()
        } else {
            process::Command::new("ssh")
                .args(&[
                    &*user_at_host,
                    "mkdir",
                    "-p",
                    &*format!("{}/dinghy", prefix),
                ])
                .status()
        };
        let target_path = format!(
            "{}/dinghy/{}",
            prefix,
            app.file_name().unwrap().to_str().unwrap()
        );
        info!("Rsyncing to {}", self.name());
        debug!(
            "rsync {}/ {}:{}/",
            app.to_str().unwrap(),
            user_at_host,
            &*target_path
        );
        let mut command = process::Command::new("/usr/bin/rsync");
        command.arg("-a").arg("-v");
        if let Some(port) = self.config.port {
            command.arg(&*format!("ssh -p {}", port));
        };
        command.arg(&*format!("{}/", app.to_str().unwrap()))
                .arg(&*format!("{}:{}/", user_at_host, &*target_path));
        if !log_enabled!(::log::LogLevel::Debug) {
            command.stdout(::std::process::Stdio::null()).stderr(::std::process::Stdio::null());
        }
        if !command.status()?.success() {
            Err("error installing app")?
        }
        Ok(())
    }
    fn clean_app(&self, app_path: &path::Path) -> Result<()> {
        let user_at_host = format!("{}@{}", self.config.username, self.config.hostname);
        let prefix = self.config.path.clone().unwrap_or("/tmp".into());
        let app_name = app_path.file_name().unwrap();
        let path = path::PathBuf::from(prefix).join("dinghy").join(app_name);
        let stat = if let Some(port) = self.config.port {
            process::Command::new("ssh")
                .arg(user_at_host)
                .arg("-p")
                .arg(&*format!("{}", port))
                .arg(&*format!("rm -rf {}", &path.to_str().unwrap()))
                .status()?
        } else {
            process::Command::new("ssh")
                .arg(user_at_host)
                .arg(&*format!("rm -rf {}", &path.to_str().unwrap()))
                .status()?
        };
        if !stat.success() {
            Err("test fail.")?
        }
        Ok(())
    }
    fn run_app(&self, app_path: &path::Path, args: &[&str], envs: &[&str]) -> Result<()> {
        let user_at_host = format!("{}@{}", self.config.username, self.config.hostname);
        let prefix = self.config.path.clone().unwrap_or("/tmp".into());
        let app_name = app_path.file_name().unwrap();
        let path = path::PathBuf::from(prefix).join("dinghy").join(app_name);
        let exe = path.join(&app_name);
        let mut command = process::Command::new("ssh");
        if let Some(port) = self.config.port {
            command.arg("-p").arg(&*format!("{}", port));
        }
        if ::isatty::stdout_isatty() {
            command.arg("-t").arg("-o").arg("LogLevel=QUIET");
        }
        command.arg(user_at_host).arg(&*format!(
            "cd {:?} ; DINGHY=1 {} {}",
            path,
            envs.join(" "),
            &exe.to_str().unwrap()
        )).args(args);
        let stat = command.status()?;
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
