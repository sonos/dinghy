use crate::compiler::Compiler;
use crate::config::PlatformConfiguration;
use dinghy_build::build_env::set_all_env;
use crate::overlay::Overlayer;
use crate::platform;
use crate::project::Project;
use std::fmt::{Debug, Formatter};
use std::process::Command;
use std::sync::Arc;
use crate::Build;
use crate::BuildArgs;
use crate::Device;
use crate::Platform;
use crate::Result;

#[derive(Clone)]
pub struct HostPlatform {
    compiler: Arc<Compiler>,
    pub configuration: PlatformConfiguration,
    pub id: String,
}

impl HostPlatform {
    pub fn new(
        compiler: Arc<Compiler>,
        configuration: PlatformConfiguration,
    ) -> Result<Box<dyn Platform>> {
        Ok(Box::new(HostPlatform {
            compiler: compiler,
            configuration,
            id: "host".to_string(),
        }))
    }
}

impl Debug for HostPlatform {
    fn fmt(&self, fmt: &mut Formatter) -> ::std::fmt::Result {
        write!(fmt, "{}", self.id)
    }
}

impl Platform for HostPlatform {
    fn build(&self, project: &Project, build_args: &BuildArgs) -> Result<Build> {
        // Set custom env variables specific to the platform
        set_all_env(&self.configuration.env());

        Overlayer::overlay(&self.configuration, self, project, "/")?;

        self.compiler.build(None, build_args)
    }

    fn id(&self) -> String {
        "host".to_string()
    }

    fn is_compatible_with(&self, device: &dyn Device) -> bool {
        device.is_compatible_with_host_platform(self)
    }

    fn rustc_triple(&self) -> Option<&str> {
        None
    }

    fn strip(&self, build: &Build) -> Result<()> {
        for runnable in &build.runnables {
            info!("Stripping {}", runnable.exe.display());
            platform::strip_runnable(runnable, Command::new("strip"))?;
        }
        Ok(())
    }
}
