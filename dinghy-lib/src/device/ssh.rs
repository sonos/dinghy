use config::{Configuration, SshDeviceConfiguration};
use errors::*;
use platform::regular_platform::RegularPlatform;
use project::Project;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use DeviceCompatibility;
use {Device, PlatformManager};
use Build;
use Platform;
use Runnable;

#[derive(Debug, Clone)]
pub struct SshDevice {
    id: String,
    conf: SshDeviceConfiguration,
}

impl DeviceCompatibility for SshDevice {
    fn is_compatible_with_regular_platform(&self, platform: &RegularPlatform) -> bool {
        self.conf.platform.as_ref().map_or(false, |it| *it == platform.id)
    }
}

impl Device for SshDevice {
    fn name(&self) -> &str {
        &*self.id
    }
    fn id(&self) -> &str {
        &*self.id
    }
    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
    }
    fn make_app(&self, project: &Project, build: &Build, runnable: &Runnable) -> Result<PathBuf> {
        let app_name = "dinghy";
        let app_path = runnable.exe.parent()
            .ok_or(format!("Invalid executable file {}", &runnable.exe.display()))?
            .join("dinghy").join(app_name);
        let exe_path = app_path.join(app_name);

        debug!("Making bundle {:?} for {:?}", app_path, &runnable.exe);
        fs::create_dir_all(&app_path)
            .chain_err(|| format!("Couldn't create {}", &app_path.display()))?;
        fs::copy(&runnable.exe, &exe_path)
            .chain_err(|| format!("Couldn't copy {} to {}", &runnable.exe.display(), &exe_path.display()))?;

        debug!("Copying dynamic libs to bundle");
        for dynamic_lib in &build.dynamic_libraries {
            let lib_path = app_path.join(dynamic_lib.file_name()
                .ok_or(format!("Invalid file name '{:?}'", dynamic_lib.file_name()))?);
            let _ = fs::remove_file(&lib_path); // Try to remove file first as libs are likely read-only
            trace!("Copying dynamic lib '{}'", lib_path.display());
            fs::copy(&dynamic_lib, &lib_path)
                .chain_err(|| format!("Couldn't copy {} to {}", dynamic_lib.display(), &lib_path.display()))?;
        }

        debug!("Copying src to bundle");
        project.rec_copy(&runnable.source, &app_path, false)?;

        debug!("Copying test_data to bundle");
        project.copy_test_data(&app_path)?;
        Ok(app_path.into())
    }
    fn install_app(&self, app: &Path) -> Result<()> {
        let user_at_host = format!("{}@{}", self.conf.username, self.conf.hostname);
        let prefix = self.conf.path.clone().unwrap_or("/tmp".into());
        let _stat = if let Some(port) = self.conf.port {
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
        if let Some(port) = self.conf.port {
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
    fn clean_app(&self, app_path: &Path) -> Result<()> {
        let user_at_host = format!("{}@{}", self.conf.username, self.conf.hostname);
        let prefix = self.conf.path.clone().unwrap_or("/tmp".into());
        let app_name = app_path.file_name().unwrap();
        let path = PathBuf::from(prefix).join("dinghy").join(app_name);
        let stat = if let Some(port) = self.conf.port {
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
    fn platform(&self) -> Result<Box<Platform>> {
        unimplemented!()
    }
    fn run_app(&self, app_path: &Path, args: &[&str], envs: &[&str]) -> Result<()> {
        let user_at_host = format!("{}@{}", self.conf.username, self.conf.hostname);
        let prefix = self.conf.path.clone().unwrap_or("/tmp".into());
        let app_name = app_path.file_name().unwrap();
        let path = PathBuf::from(prefix).join("dinghy").join(app_name);
        let exe = path.join(&app_name);
        let mut command = process::Command::new("ssh");
        if let Some(port) = self.conf.port {
            command.arg("-p").arg(&*format!("{}", port));
        }
        if ::isatty::stdout_isatty() {
            command.arg("-t").arg("-o").arg("LogLevel=QUIET");
        }

        command
            .arg(user_at_host)
            .arg(&*format!(
                "cd {:?} ; DINGHY=1 {} LD_LIBRARY_PATH='{}' {}",
                path,
                envs.join(" "),
                // TODO Cleanup env management
                &exe.parent().unwrap().to_str().unwrap(),
                &exe.to_str().unwrap()
            ))
            .args(args);
        let stat = command.status()?;
        if !stat.success() {
            Err("test fail.")?
        }
        Ok(())
    }
    fn debug_app(&self, _app_path: &Path, _args: &[&str], _envs: &[&str]) -> Result<()> {
        unimplemented!()
    }
}

impl Display for SshDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Ok(fmt.write_str(format!("Ssh {{ \"id\": \"{}\", \"hostname\": \"{}\", \"username\": \"{}\", \"port\": \"{}\" }}",
                                 self.id,
                                 self.conf.hostname,
                                 self.conf.username,
                                 self.conf.port.as_ref().map_or("none".to_string(), |it| it.to_string())).as_str())?)
    }
}

pub struct SshDeviceManager {
    conf: Arc<Configuration>
}

impl SshDeviceManager {
    pub fn probe(conf: Arc<Configuration>) -> Option<SshDeviceManager> {
        Some(SshDeviceManager { conf })
    }
}

impl PlatformManager for SshDeviceManager {
    fn devices(&self) -> Result<Vec<Box<Device>>> {
        Ok(self.conf.ssh_devices
            .iter()
            .map(|(k, conf)| {
                Box::new(SshDevice {
                    id: k.clone(),
                    conf: conf.clone(),
                }) as _
            })
            .collect())
    }
}
