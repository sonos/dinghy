use compiler::Compiler;
use config::PlatformConfiguration;
use dinghy_helper::build_env::set_all_env;
use overlay::Overlayer;
use overlay::overlay_work_dir;
use std::sync::Arc;
use Build;
use BuildArgs;
use Device;
use Platform;
use Result;

#[derive(Clone)]
pub struct HostPlatform {
    compiler: Arc<Compiler>,
    pub configuration: PlatformConfiguration,
    pub id: String,
}

impl HostPlatform {
    pub fn new(compiler: &Arc<Compiler>, configuration: PlatformConfiguration) -> Result<Box<Platform>> {
        Ok(Box::new(HostPlatform {
            compiler: compiler.clone(),
            configuration,
            id: "host".to_string(),
        }))
    }
}

impl Platform for HostPlatform {
    fn build(&self, build_args: BuildArgs) -> Result<Build> {
        // Set custom env variables specific to the platform
        set_all_env(&self.configuration.env());

        Overlayer::new(self, "/", overlay_work_dir(&self.compiler, self)?)
            .overlay(&self.configuration, self.compiler.project_dir()?)?;

        self.compiler.build(None, build_args)
    }

    fn id(&self) -> String {
        "host".to_string()
    }

    fn is_compatible_with(&self, device: &Device) -> bool {
        device.is_compatible_with_host_platform(self)
    }

    fn rustc_triple(&self) -> Option<&str> {
        None
    }
}
