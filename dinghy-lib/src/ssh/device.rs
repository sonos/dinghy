use crate::config::SshDeviceConfiguration;
use crate::device::make_remote_app;
use crate::errors::*;
use crate::platform::regular_platform::RegularPlatform;
use crate::project::Project;
use crate::utils::path_to_str;
use crate::Build;
use crate::BuildBundle;
use crate::Device;
use crate::DeviceCompatibility;
use crate::Runnable;
use std::fmt;
use std::fmt::Formatter;
use std::fmt::{Debug, Display};
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

pub struct SshDevice {
    pub id: String,
    pub conf: SshDeviceConfiguration,
}

impl SshDevice {
    fn install_app(
        &self,
        project: &Project,
        build: &Build,
        runnable: &Runnable,
    ) -> Result<(BuildBundle, BuildBundle)> {
        debug!("make_remote_app {}", runnable.id);
        let build_bundle = make_remote_app(project, build, runnable)?;
        trace!("make_remote_app {} done", runnable.id);
        let remote_bundle = self.to_remote_bundle(&build_bundle)?;
        trace!("Create remote dir: {:?}", remote_bundle.bundle_dir);

        let _ = self
            .ssh_command()?
            .arg("mkdir")
            .arg("-p")
            .arg(&remote_bundle.bundle_dir)
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
        if atty::is(atty::Stream::Stdout) {
            command.arg("-t").arg("-o").arg("LogLevel=QUIET");
        }
        Ok(command)
    }

    fn sync_rsync(&self, rsync: Option<String>) -> Result<String> {
        match rsync {
            Some(rsync) => {
                let rsync_path = "/tmp/rsync";
                let mut command = Command::new("scp");
                command.arg("-q");
                command.arg(format!("{}", rsync));
                command.arg(format!(
                    "{}@{}:{}",
                    self.conf.username, self.conf.hostname, rsync_path
                ));
                if let Some(port) = self.conf.port {
                    command.arg("-p").arg(&format!("{}", port));
                }
                debug!("Running {:?}", command);
                if !command.status()?.success() {
                    bail!("Error copying rsync binary ({:?})", command)
                }
                Ok(rsync_path.to_string())
            }
            None => Ok("/usr/bin/rsync".to_string()),
        }
    }

    fn sync<FP: AsRef<Path>, TP: AsRef<Path>>(&self, from_path: FP, to_path: TP) -> Result<()> {
        let rsync = self.sync_rsync(self.conf.install_adhoc_rsync_local_path.clone());
        let rsync = match rsync {
            Ok(rsync_path) => rsync_path,
            Err(error) => bail!("Problem with rsync on the target: {:?}", error),
        };
        let mut command = Command::new("/usr/bin/rsync");
        command.arg(&format!("--rsync-path={}", rsync));
        command.arg("-a").arg("-v");
        if let Some(port) = self.conf.port {
            command.arg(&*format!("ssh -p {}", port));
        };
        if !log_enabled!(::log::Level::Debug) {
            command.stdout(::std::process::Stdio::null());
            command.stderr(::std::process::Stdio::null());
        }
        command
            .arg(&format!("{}/", path_to_str(&from_path.as_ref())?))
            .arg(&format!(
                "{}@{}:{}/",
                self.conf.username,
                self.conf.hostname,
                path_to_str(&to_path.as_ref())?
            ));
        debug!("Running {:?}", command);
        if !command.status()?.success() {
            bail!("Error syncing ssh directory ({:?})", command)
        } else {
            Ok(())
        }
    }

    fn to_remote_bundle(&self, build_bundle: &BuildBundle) -> Result<BuildBundle> {
        let remote_prefix =
            PathBuf::from(self.conf.path.clone().unwrap_or("/tmp".into())).join("dinghy");
        build_bundle.replace_prefix_with(remote_prefix)
    }
}

impl DeviceCompatibility for SshDevice {
    fn is_compatible_with_regular_platform(&self, platform: &RegularPlatform) -> bool {
        self.conf
            .platform
            .as_ref()
            .map_or(false, |it| *it == platform.id)
    }
}

impl Device for SshDevice {
    fn clean_app(&self, build_bundle: &BuildBundle) -> Result<()> {
        let status = self
            .ssh_command()?
            .arg(&format!(
                "rm -rf {}",
                path_to_str(&build_bundle.bundle_exe)?
            ))
            .status()?;
        if !status.success() {
            bail!("test fail.")
        }
        Ok(())
    }

    fn debug_app(
        &self,
        _project: &Project,
        _build: &Build,
        _args: &[&str],
        _envs: &[&str],
    ) -> Result<BuildBundle> {
        unimplemented!()
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.id
    }

    fn run_app(
        &self,
        project: &Project,
        build: &Build,
        args: &[&str],
        envs: &[&str],
    ) -> Result<Vec<BuildBundle>> {
        let mut build_bundles = vec![];
        let args: Vec<String> = args
            .iter()
            .map(|&a| ::shell_escape::escape(a.into()).to_string())
            .collect();
        for runnable in &build.runnables {
            info!("Install {:?}", runnable.id);
            let (build_bundle, remote_bundle) = self.install_app(&project, &build, &runnable)?;
            debug!("Installed {:?}", runnable.id);
            let command = format!(
                "cd '{}' ; {} RUST_BACKTRACE=1 DINGHY=1 LD_LIBRARY_PATH=\"{}:$LD_LIBRARY_PATH\" {} {} {}",
                path_to_str(&remote_bundle.bundle_dir)?,
                envs.join(" "),
                path_to_str(&remote_bundle.lib_dir)?,
                path_to_str(&remote_bundle.bundle_exe)?,
                if build.build_args.compile_mode == ::cargo::core::compiler::CompileMode::Bench { "--bench" } else { "" },
                args.join(" ")
                );
            trace!("Ssh command: {}", command);
            info!(
                "Run {} on {} ({:?})",
                runnable.id, self.id, build.build_args.compile_mode
            );

            let status = self.ssh_command()?.arg(&command).status()?;
            if !status.success() {
                bail!("Test failed 🐛")
            }

            build_bundles.push(build_bundle);
        }
        Ok(build_bundles)
    }

    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
    }
}

impl Debug for SshDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Ok(fmt.write_str(format!("Ssh {{ \"id\": \"{}\", \"hostname\": \"{}\", \"username\": \"{}\", \"port\": \"{}\" }}",
                                 self.id,
                                 self.conf.hostname,
                                 self.conf.username,
                                 self.conf.port.as_ref().map_or("none".to_string(), |it| it.to_string())).as_str())?)
    }
}

impl Display for SshDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "{}", self.conf.hostname)
    }
}
