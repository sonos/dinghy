mod device;
use std::sync;
use crate::{Configuration, Device, Platform, PlatformManager, Result};

use self::device::SshDevice;

pub struct SshDeviceManager {
    conf: sync::Arc<Configuration>,
}

impl SshDeviceManager {
    pub fn probe(conf: sync::Arc<Configuration>) -> Option<SshDeviceManager> {
        Some(SshDeviceManager { conf })
    }
}

impl PlatformManager for SshDeviceManager {
    fn devices(&self) -> Result<Vec<Box<dyn Device>>> {
        Ok(self
            .conf
            .ssh_devices
            .iter()
            .map(|(k, conf)| {
                Box::new(SshDevice {
                    id: k.clone(),
                    conf: conf.clone(),
                }) as _
            })
            .collect())
    }
    fn platforms(&self) -> Result<Vec<Box<dyn Platform>>> {
        Ok(vec![])
    }
}
