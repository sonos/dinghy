use config::{Configuration, SshDeviceConfiguration};
use errors::*;
use device::make_remote_app;
use platform::regular_platform::RegularPlatform;
use project::Project;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use utils::path_to_str;
use Build;
use Device;
use DeviceCompatibility;
use PlatformManager;
use BuildBundle;
use Runnable;

pub struct SshDevice {
    id: String,
    conf: SshDeviceConfiguration,
}

impl SshDevice {
    fn install_app(&self, project: &Project, build: &Build, runnable: &Runnable) -> Result<(BuildBundle, BuildBundle)> {
        let build_bundle = make_remote_app(project, build, runnable)?;
        let remote_bundle = self.to_remote_bundle(&build_bundle)?;

        let _ = self.ssh_command()?
            .arg("mkdir").arg("-p").arg(&remote_bundle.bundle_dir)
            .status();

        info!("Install {} to {}", runnable.id, self.id);
        self.sync(&build_bundle.bundle_dir, &remote_bundle.bundle_dir)?;
        self.sync(&build_bundle.lib_dir, &remote_bundle.lib_dir)?;
        Ok((build_bundle, remote_bundle))
    }

    fn ssh_command(&self) -> Result<Command> {
        let mut command = Command::new("ssh");
        command.arg(format!("{}@{}", self.conf.username, self.conf.hostname));
        if let Some(port) = self.conf.port {
            command.arg("-p").arg(&format!("{}", port));
        }
        if ::isatty::stdout_isatty() {
            command.arg("-t").arg("-o").arg("LogLevel=QUIET");
        }
        Ok(command)
    }

    fn sync<FP: AsRef<Path>, TP: AsRef<Path>>(&self, from_path: FP, to_path: TP) -> Result<()> {
        let mut command = Command::new("/usr/bin/rsync");
        command.arg("-a").arg("-v");
        if let Some(port) = self.conf.port {
            command.arg(&*format!("ssh -p {}", port));
        };
        if !log_enabled!(::log::LogLevel::Debug) {
            command.stdout(::std::process::Stdio::null());
            command.stderr(::std::process::Stdio::null());
        }
        command
            .arg(&format!("{}/", path_to_str(&from_path.as_ref())?))
            .arg(&format!("{}@{}:{}/", self.conf.username, self.conf.hostname, path_to_str(&to_path.as_ref())?));
        debug!("Running {:?}", command);
        if !command.status()?.success() {
            bail!("Error syncing ssh directory ({:?})", command)
        } else {
            Ok(())
        }
    }

    fn to_remote_bundle(&self, build_bundle: &BuildBundle) -> Result<BuildBundle> {
        let remote_prefix = PathBuf::from(self.conf.path.clone()
            .unwrap_or("/tmp".into()))
            .join("dinghy");
        build_bundle.replace_prefix_with(remote_prefix)
    }
}

impl DeviceCompatibility for SshDevice {
    fn is_compatible_with_regular_platform(&self, platform: &RegularPlatform) -> bool {
        self.conf.platform.as_ref().map_or(false, |it| *it == platform.id)
    }
}

impl Device for SshDevice {
    fn clean_app(&self, build_bundle: &BuildBundle) -> Result<()> {
        let status = self.ssh_command()?
            .arg(&format!("rm -rf {}", path_to_str(&build_bundle.bundle_exe)?))
            .status()?;
        if !status.success() {
            Err("test fail.")?
        }
        Ok(())
    }

    fn debug_app(&self, _project: &Project, _build: &Build, _args: &[&str], _envs: &[&str]) -> Result<BuildBundle> {
        unimplemented!()
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.id
    }

    fn run_app(&self, project: &Project, build: &Build, args: &[&str], envs: &[&str]) -> Result<Vec<BuildBundle>> {
        let mut build_bundles = vec![];
        let args:Vec<String> = args.iter().map(|&a| ::shell_escape::escape(a.into()).to_string()).collect();
        for runnable in &build.runnables {
            let (build_bundle, remote_bundle) = self.install_app(&project, &build, &runnable)?;
            let command = format!(
                "cd '{}' ; {} RUST_BACKTRACE=1 DINGHY=1 LD_LIBRARY_PATH=\"{}:$LD_LIBRARY_PATH\" {} {} {}",
                path_to_str(&remote_bundle.bundle_dir)?,
                envs.join(" "),
                path_to_str(&remote_bundle.lib_dir)?,
                path_to_str(&remote_bundle.bundle_exe)?,
                if build.build_args.compile_mode == ::cargo::ops::CompileMode::Bench { "--bench" } else { "" },
                args.join(" ")
                );
            info!("Running {} on {} ({:?})", runnable.id, self.id, build.build_args.compile_mode);

            let status = self.ssh_command()?
                .arg(&command)
                .status()?;
            if !status.success() {
                Err("Test failed 🐛")?
            }

            build_bundles.push(build_bundle);
        }
        Ok(build_bundles)
    }

    fn start_remote_lldb(&self) -> Result<String> {
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
