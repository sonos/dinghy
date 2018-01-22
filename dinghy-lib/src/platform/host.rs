use config::PlatformConfiguration;
use Platform;
use Result;

#[derive(Debug, Clone)]
pub struct HostPlatform {
    pub configuration: PlatformConfiguration,
    pub id: String,
}

impl HostPlatform {
    pub fn new() -> Result<Box<Platform>> {
        Ok(Box::new(HostPlatform {
            configuration: PlatformConfiguration {
                env: None,
                overlays: None,
                rustc_triple: None,
                sysroot: None,
                toolchain: None,
            },
            id: "host".to_string(),
        }))
    }
}
