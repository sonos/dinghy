use platform::host::HostPlatform;
use project::Project;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use Build;
use BuildBundle;
use Device;
use PlatformManager;
use DeviceCompatibility;
use Result;
use Runnable;

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


#[derive(Debug)]
pub struct HostDevice {}

impl HostDevice {
    pub fn new() -> Self {
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

    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
    }

    fn install_app(&self, _project: &Project, _build: &Build, _runnable: &Runnable) -> Result<BuildBundle> {
        unimplemented!()
    }

    fn clean_app(&self, _build_bundle: &BuildBundle) -> Result<()> {
        unimplemented!()
    }

    fn run_app(&self, _build_bundle: &BuildBundle, _args: &[&str], _envs: &[&str]) -> Result<()> {
        unimplemented!()
    }

    fn debug_app(&self, _build_bundle: &BuildBundle, _args: &[&str], _envs: &[&str]) -> Result<()> {
        unimplemented!()
    }
}

impl Display for HostDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Ok(fmt.write_str(format!("Host {{ }}").as_str())?)
    }
}

impl DeviceCompatibility for HostDevice {
    fn is_compatible_with_host_platform(&self, _platform: &HostPlatform) -> bool {
        true
    }
}
