use std::{path, process, sync};
use errors::*;
use {Device, PlatformManager, Platform};

use config::{ Configuration, SshDeviceConfiguration};
use PlatformCompatibility;
use regular_platform::RegularPlatform;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug, Clone)]
pub struct SshDevice {
    id: String,
    conf: sync::Arc<Configuration>,
}

impl SshDevice {
    fn ssh_config(&self) -> &SshDeviceConfiguration {
        &self.conf.ssh_devices[&self.id]
    }
}

impl PlatformCompatibility for SshDevice {
    fn is_compatible_with_regular_platform(&self, platform: &RegularPlatform) -> bool {
        self.ssh_config().platform.as_ref().map_or(false, |it| *it == platform.id)
    }
}

impl Device for SshDevice {
    fn name(&self) -> &str {
        &*self.id
    }
    fn id(&self) -> &str {
        &*self.id
    }
    fn rustc_triple_guess(&self) -> Option<String> {
        None
    }
    fn platform(&self) -> Result<Box<Platform>> {
        debug!("building platform for {}", self);
        match self.ssh_config().platform {
            Some(ref pf_name) => {
            let pf = &self.conf.platforms.get(pf_name).ok_or(format!("platform {} not found", pf_name))?;
            ::regular_platform::RegularPlatform::new(self.id.clone(), pf.rustc_triple.clone().unwrap(), pf.toolchain.clone().unwrap())
            },
            None => {
                let tc = self.ssh_config().toolchain.clone().ok_or(format!("device {} has neither platform nor toolchain specified", self.name()))?;
                let target = self.ssh_config().target.clone().ok_or(format!("device {} has neither platform nor target specified", self.name()))?;
                ::regular_platform::RegularPlatform::new(self.id.clone(), target, tc)
            }
        }
    }
    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
    }
    fn make_app(&self, source: &path::Path, exe: &path::Path) -> Result<path::PathBuf> {
        ::make_linux_app(source, exe)
    }
    fn install_app(&self, app: &path::Path) -> Result<()> {
        let user_at_host = format!("{}@{}", self.ssh_config().username, self.ssh_config().hostname);
        let prefix = self.ssh_config().path.clone().unwrap_or("/tmp".into());
        let _stat = if let Some(port) = self.ssh_config().port {
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
        if let Some(port) = self.ssh_config().port {
            command.arg(&*format!("ssh -p {}", port));
        };
        command
            .arg(&*format!("{}/", app.to_str().unwrap()))
            .arg(&*format!("{}:{}/", user_at_host, &*target_path));
        if !log_enabled!(::log::LogLevel::Debug) {
            command
                .stdout(::std::process::Stdio::null())
                .stderr(::std::process::Stdio::null());
        }
        if !command.status()?.success() {
            Err("error installing app")?
        }
        Ok(())
    }
    fn clean_app(&self, app_path: &path::Path) -> Result<()> {
        let user_at_host = format!("{}@{}", self.ssh_config().username, self.ssh_config().hostname);
        let prefix = self.ssh_config().path.clone().unwrap_or("/tmp".into());
        let app_name = app_path.file_name().unwrap();
        let path = path::PathBuf::from(prefix).join("dinghy").join(app_name);
        let stat = if let Some(port) = self.ssh_config().port {
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
        let user_at_host = format!("{}@{}", self.ssh_config().username, self.ssh_config().hostname);
        let prefix = self.ssh_config().path.clone().unwrap_or("/tmp".into());
        let app_name = app_path.file_name().unwrap();
        let path = path::PathBuf::from(prefix).join("dinghy").join(app_name);
        let exe = path.join(&app_name);
        let mut command = process::Command::new("ssh");
        if let Some(port) = self.ssh_config().port {
            command.arg("-p").arg(&*format!("{}", port));
        }
        if ::isatty::stdout_isatty() {
            command.arg("-t").arg("-o").arg("LogLevel=QUIET");
        }
        command
            .arg(user_at_host)
            .arg(&*format!(
                "cd {:?} ; DINGHY=1 {} {}",
                path,
                envs.join(" "),
                &exe.to_str().unwrap()
            ))
            .args(args);
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

impl Display for SshDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        let ssh_config = self.ssh_config();
        Ok(fmt.write_str(format!("Ssh {{ \"id\": \"{}\", \"hostname\": \"{}\", \"username\": \"{}\", \"port\": \"{}\" }}",
                                 self.id,
                                 ssh_config.hostname,
                                 ssh_config.username,
                                 ssh_config.port.as_ref().map_or("none".to_string(), |it| it.to_string())).as_str())?)
    }
}

pub struct SshDeviceManager {
    conf: sync::Arc<Configuration>
}

impl SshDeviceManager {
    pub fn probe(conf: sync::Arc<Configuration>) -> Option<SshDeviceManager> {
        Some(SshDeviceManager {conf})
    }
}

impl PlatformManager for SshDeviceManager {
    fn devices(&self) -> Result<Vec<Box<Device>>> {
        Ok(self.conf.ssh_devices
            .iter()
            .map(|(k, _)| {
                Box::new(SshDevice {
                    id: k.clone(),
                    conf: self.conf.clone(),
                }) as _
            })
            .collect())
    }
}
