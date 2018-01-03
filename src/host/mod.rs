use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::path::Path;
use std::path::PathBuf;

use Device;
use Platform;
use PlatformManager;
use PlatformCompatibility;
use Result;

pub struct HostManager {}

impl HostManager {
    pub fn probe() -> Option<HostManager> {
        Some(HostManager {})
    }
}

impl PlatformManager for HostManager {
    fn devices(&self) -> Result<Vec<Box<Device>>> {
        Ok(vec![Box::new(HostDevice::new())])
    }
}


#[derive(Debug, Clone)]
pub struct HostPlatform {}

impl HostPlatform {
    pub fn new() -> Result<Box<Platform>> {
        Ok(Box::new(HostPlatform {}))
    }
}

#[derive(Debug)]
pub struct HostDevice {}

impl HostDevice {
    fn new() -> Self {
        HostDevice {}
    }
}

impl Device for HostDevice {
    fn name(&self) -> &str {
        "host device"
    }

    fn id(&self) -> &str {
        "HOST"
    }

    fn rustc_triple_guess(&self) -> Option<String> {
        None
    }

    fn platform(&self) -> Result<Box<Platform>> {
        HostPlatform::new()
    }

    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
    }

    fn make_app(&self, _source: &Path, _app: &Path) -> Result<PathBuf> {
        unimplemented!()
    }

    fn install_app(&self, _path: &Path) -> Result<()> {
        unimplemented!()
    }

    fn clean_app(&self, _path: &Path) -> Result<()> {
        unimplemented!()
    }

    fn run_app(&self, _app: &Path, _args: &[&str], _envs: &[&str]) -> Result<()> {
        unimplemented!()
    }

    fn debug_app(&self, _app: &Path, _args: &[&str], _envs: &[&str]) -> Result<()> {
        unimplemented!()
    }
}

impl Display for HostDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Ok(fmt.write_str(format!("Host {{ }}").as_str())?)
    }
}

impl PlatformCompatibility for HostDevice {
    fn is_compatible_with_host_platform(&self, _platform: &HostPlatform) -> bool {
        true
    }
}

impl Platform for HostPlatform {
    fn id(&self) -> String {
        unimplemented!()
    }

    fn cc_command(&self) -> Result<String> {
        unimplemented!()
    }

    fn linker_command(&self) -> Result<String> {
        unimplemented!()
    }

    fn rustc_triple(&self) -> Result<String> {
        unimplemented!()
    }

    fn setup_env(&self) -> Result<()> {
        unimplemented!()
    }

    fn setup_more_env(&self) -> Result<()> {
        unimplemented!()
    }

    fn is_compatible_with(&self, device: &Device) -> bool {
        device.is_compatible_with_host_platform(self)
    }
}