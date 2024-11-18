use crate::{Configuration, Device, Platform, PlatformManager, Result};
use std::sync;

mod device;

pub use self::device::ScriptDevice;

pub struct ScriptDeviceManager {
    conf: sync::Arc<Configuration>,
}

impl ScriptDeviceManager {
    pub fn probe(conf: sync::Arc<Configuration>) -> Option<ScriptDeviceManager> {
        Some(ScriptDeviceManager { conf })
    }
}

impl PlatformManager for ScriptDeviceManager {
    fn devices(&self) -> Result<Vec<Box<dyn Device>>> {
        Ok(self
            .conf
            .script_devices
            .iter()
            .map(|(k, conf)| {
                Box::new(ScriptDevice {
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
