use crate::config::SshDeviceConfiguration;
use crate::device::make_remote_app;
use crate::errors::*;
use crate::host::HostPlatform;
use crate::platform::regular_platform::RegularPlatform;
use crate::project::Project;
use crate::utils::{get_current_verbosity, path_to_str, user_facing_log, LogCommandExt};
use crate::Build;
use crate::BuildBundle;
use crate::Device;
use crate::DeviceCompatibility;
use std::fmt;
use std::fmt::Formatter;
use std::fmt::{Debug, Display};
use std::io::IsTerminal;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

#[derive(Clone)]
pub struct SshDevice {
    pub id: String,
    pub conf: SshDeviceConfiguration,
}

impl SshDevice {
    fn install_app(&self, project: &Project, build: &Build) -> Result<(BuildBundle, BuildBundle)> {
        user_facing_log(
            "Installing",
            &format!("{} to {}", build.runnable.id, self.id),
            0,
        );

        log::debug!("make_remote_app {}", build.runnable.id);
        let build_bundle = make_remote_app(project, build)?;

        log::trace!("make_remote_app {} done", build.runnable.id);
        let remote_bundle = self.to_remote_bundle(&build_bundle)?;
        log::trace!("Create remote dir: {:?}", remote_bundle.bundle_dir);

        let _ = self
            .ssh_command()?
            .arg("mkdir")
            .arg("-p")
            .arg(&remote_bundle.bundle_dir)
            .log_invocation(2)
            .status();

        log::info!("Install {} to {}", build.runnable.id, self.id);
        self.sync(&build_bundle.bundle_dir, &remote_bundle.bundle_dir)?;
        self.sync(&build_bundle.lib_dir, &remote_bundle.lib_dir)?;
        Ok((build_bundle, remote_bundle))
    }

    fn ssh_command(&self) -> Result<Command> {
        let mut command = Command::new("ssh");
        if let Some(port) = self.conf.port {
            command.arg("-p").arg(&format!("{}", port));
        }
        if std::io::stdout().is_terminal() {
            command.arg("-t").arg("-o").arg("LogLevel=QUIET");
        }
        command.arg(format!("{}@{}", self.conf.username, self.conf.hostname));
        Ok(command)
    }

    fn sync_rsync(&self) -> Result<String> {
        match &self.conf.install_adhoc_rsync_local_path {
            Some(rsync) => {
                let rsync_path = PathBuf::from(self.conf.path.clone().unwrap_or("/tmp".into()))
                    .join("dinghy")
                    .join("rsync");
                let rsync_path = rsync_path
                    .to_str()
                    .ok_or_else(|| anyhow!("Could not format rsync remote path"))?;

                if self
                    .ssh_command()?
                    .arg("[")
                    .arg("-f")
                    .arg(rsync_path)
                    .arg("]")
                    .log_invocation(2)
                    .status()?
                    .success()
                {
                    log::debug!("ad-hoc rsync already present on device, skipping copy")
                } else {
                    let mut command = Command::new("scp");
                    if let Some(true) = self.conf.use_legacy_scp_protocol_for_adhoc_rsync_copy {
                        command.arg("-O");
                    }
                    command.arg("-q");
                    if let Some(port) = self.conf.port {
                        command.arg("-P").arg(&format!("{}", port));
                    }
                    command.arg(format!("{}", rsync));
                    command.arg(format!(
                        "{}@{}:{}",
                        self.conf.username, self.conf.hostname, rsync_path
                    ));
                    log::debug!("Running {:?}", command);
                    if !command.log_invocation(3).status()?.success() {
                        bail!("Error copying rsync binary ({:?})", command)
                    }
                }
                Ok(rsync_path.to_string())
            }
            None => Ok("/usr/bin/rsync".to_string()),
        }
    }

    fn sync<FP: AsRef<Path>, TP: AsRef<Path>>(&self, from_path: FP, to_path: TP) -> Result<()> {
        let rsync = self.sync_rsync();
        let rsync = match rsync {
            Ok(rsync_path) => rsync_path,
            Err(error) => bail!("Problem with rsync on the target: {:?}", error),
        };
        let mut command = Command::new("rsync");
        command.arg(&format!("--rsync-path={}", rsync));
        command.arg("-a").arg("-v");
        if let Some(port) = self.conf.port {
            command.arg("-e").arg(&*format!("ssh -p {}", port));
        };
        if !log::log_enabled!(::log::Level::Debug) {
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
        log::debug!("Running {:?}", command);
        if !command
            .log_invocation(1)
            .status()
            .with_context(|| format!("failed to run '{:?}'", command))?
            .success()
        {
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

    fn is_compatible_with_host_platform(&self, platform: &HostPlatform) -> bool {
        self.conf
            .platform
            .as_ref()
            .map_or(true, |it| *it == platform.id)
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
            .log_invocation(1)
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
    ) -> Result<BuildBundle> {
        let remote_shell_vars_as_context = |a: &str| -> Option<std::borrow::Cow<str>> {
            self.conf.remote_shell_vars.get(a).map(|s| s.into())
        };
        let args: Vec<String> = args
            .iter()
            .map(|&a| {
                shellexpand::full_with_context_no_errors(
                    a,
                    || remote_shell_vars_as_context("HOME"),
                    remote_shell_vars_as_context,
                )
            })
            .map(|a| ::shell_escape::escape(a).to_string())
            .collect();
        log::info!("Install {:?}", build.runnable.id);
        let (build_bundle, remote_bundle) = self.install_app(&project, &build)?;
        log::debug!("Installed {:?}", build.runnable.id);
        let command = format!(
            "cd '{}' ; RUST_BACKTRACE=1 {} DINGHY=1 LD_LIBRARY_PATH=\"{}:$LD_LIBRARY_PATH\" {} {}",
            path_to_str(&remote_bundle.bundle_dir)?,
            envs.join(" "),
            path_to_str(&remote_bundle.lib_dir)?,
            path_to_str(&remote_bundle.bundle_exe)?,
            args.join(" ")
        );
        log::trace!("Ssh command: {}", command);
        log::info!("Run {} on {}", build.runnable.id, self.id,);
        if get_current_verbosity() < 1 {
            // we log the full command for verbosity > 1, just log a short message when the user
            // didn't ask for verbose output
            user_facing_log(
                "Running",
                &format!("{} on {}", build.runnable.id, self.id),
                0,
            );
        }

        let status = self
            .ssh_command()?
            .arg(&command)
            .log_invocation(1)
            .status()?;
        if !status.success() {
            bail!("Failed")
        }

        Ok(build_bundle)
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
        write!(fmt, "{} ({})", self.id, self.conf.hostname)
    }
}
