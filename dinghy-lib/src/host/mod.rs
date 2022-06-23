use crate::{Configuration, Device, Platform, PlatformConfiguration, PlatformManager, Result};

mod platform;

pub use self::platform::HostPlatform;

pub struct HostManager {
    host_conf: PlatformConfiguration,
}

impl HostManager {
    pub fn probe(conf: &Configuration) -> Option<HostManager> {
        let host_conf = conf
            .platforms
            .get("host")
            .map(|it| (*it).clone())
            .unwrap_or(PlatformConfiguration::empty());
        Some(HostManager { host_conf })
    }

    fn platform(&self) -> Result<HostPlatform> {
        platform::HostPlatform::new(self.host_conf.clone())
    }
}

impl PlatformManager for HostManager {
    fn devices(&self) -> Result<Vec<Box<dyn Device>>> {
        Ok(vec![])
    }

    fn platforms(&self) -> Result<Vec<Box<dyn Platform>>> {
        Ok(vec![Box::new(self.platform()?)])
    }
}
