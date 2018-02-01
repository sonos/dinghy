use compiler::Compiler;
use config::PlatformConfiguration;
use dinghy_helper::build_env::set_all_env;
use overlay::Overlayer;
use Build;
use BuildArgs;
use Device;
use Platform;
use Result;

#[derive(Debug, Clone)]
pub struct HostPlatform {
    pub configuration: PlatformConfiguration,
    pub id: String,
}

impl HostPlatform {
    pub fn new(configuration: PlatformConfiguration) -> Result<Box<Platform>> {
        Ok(Box::new(HostPlatform {
            configuration,
            id: "host".to_string(),
        }))
    }
}

impl Platform for HostPlatform {
    fn build(&self, compiler: &Compiler, build_args: BuildArgs) -> Result<Build> {
        // Set custom env variables specific to the platform
        set_all_env(&self.configuration.env());

        Overlayer::new(self, "/", compiler.target_dir(self.rustc_triple())?.join(&self.id))
            .overlay(&self.configuration, compiler.project_dir()?)?;

        compiler.build(None, build_args)
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
