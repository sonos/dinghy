use crate::config::PlatformConfiguration;
use crate::overlay::Overlayer;
use crate::platform;
use crate::project::Project;
use crate::utils::LogCommandExt;
use crate::Build;
use crate::Device;
use crate::Platform;
use crate::Result;
use crate::SetupArgs;
use anyhow::anyhow;
use dinghy_build::build_env::{envify, set_all_env, set_env};
use std::fmt::{Debug, Formatter};
use std::io::BufRead;
use std::process::{Command, Stdio};

#[derive(Clone)]
pub struct HostPlatform {
    pub configuration: PlatformConfiguration,
    pub id: String,
}

impl HostPlatform {
    pub fn new(configuration: PlatformConfiguration) -> Result<HostPlatform> {
        Ok(HostPlatform {
            configuration,
            id: "host".to_string(),
        })
    }
}

impl Debug for HostPlatform {
    fn fmt(&self, fmt: &mut Formatter) -> ::std::fmt::Result {
        write!(fmt, "{}", self.id)
    }
}

impl Platform for HostPlatform {
    fn setup_env(&self, project: &Project, setup_args: &SetupArgs) -> Result<()> {
        // Set custom env variables specific to the platform
        set_all_env(&self.configuration.env());

        let triple = std::process::Command::new("rustc")
            .arg("-vV")
            .stdout(Stdio::piped())
            .log_invocation(3)
            .output()?
            .stdout
            .lines()
            .find_map(|line| {
                line.ok()
                    .and_then(|line| line.strip_prefix("host: ").map(ToString::to_string))
            })
            .ok_or_else(|| anyhow!("could not get host triple from rustc"))?;

        set_env(
            format!("CARGO_TARGET_{}_RUNNER", envify(triple)),
            setup_args.get_runner_command("host"),
        );

        Overlayer::overlay(&self.configuration, self, project, "/")?;

        Ok(())
    }

    fn id(&self) -> String {
        "host".to_string()
    }

    fn is_compatible_with(&self, device: &dyn Device) -> bool {
        device.is_compatible_with_host_platform(self)
    }

    fn is_host(&self) -> bool {
        true
    }

    fn rustc_triple(&self) -> &str {
        std::env!("TARGET")
    }

    fn strip(&self, build: &mut Build) -> Result<()> {
        log::info!("Stripping {}", build.runnable.exe.display());
        build.runnable = platform::strip_runnable(&build.runnable, Command::new("strip"))?;

        Ok(())
    }

    fn sysroot(&self) -> Result<Option<std::path::PathBuf>> {
        Ok(Some(std::path::PathBuf::from("/")))
    }
}
