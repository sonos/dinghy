use crate::{
    Compiler, Configuration, Device, Platform, PlatformConfiguration, PlatformManager, Result,
};
use std::sync;

mod device;
mod platform;

pub use self::device::HostDevice;
pub use self::platform::HostPlatform;

pub struct HostManager {
    compiler: sync::Arc<Compiler>,
    host_conf: PlatformConfiguration,
}

impl HostManager {
    pub fn probe(compiler: sync::Arc<Compiler>, conf: &Configuration) -> Option<HostManager> {
        let host_conf = conf
            .platforms
            .get("host")
            .map(|it| (*it).clone())
            .unwrap_or(PlatformConfiguration::empty());
        Some(HostManager {
            compiler: compiler,
            host_conf,
        })
    }

    fn platform(&self) -> Result<HostPlatform> {
        platform::HostPlatform::new(sync::Arc::clone(&self.compiler), self.host_conf.clone())
    }
}

impl PlatformManager for HostManager {
    fn devices(&self) -> Result<Vec<Box<dyn Device>>> {
        Ok(vec![Box::new(HostDevice::new(self.platform()?, &self.compiler))])
    }

    fn platforms(&self) -> Result<Vec<Box<dyn Platform>>> {
        Ok(vec![Box::new(self.platform()?)])
    }
}
