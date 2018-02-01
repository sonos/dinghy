use compiler::Compiler;
use device::make_host_app;
use platform::host::HostPlatform;
use project::Project;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::sync::Arc;
use Build;
use BuildBundle;
use Device;
use PlatformManager;
use DeviceCompatibility;
use Result;
use Runnable;

pub struct HostManager {
    compiler: Arc<Compiler>
}

impl HostManager {
    pub fn probe(compiler: &Arc<Compiler>) -> Option<HostManager> {
        Some(HostManager {
            compiler: compiler.clone(),
        })
    }
}

impl PlatformManager for HostManager {
    fn devices(&self) -> Result<Vec<Box<Device>>> {
        Ok(vec![Box::new(HostDevice::new(&self.compiler))])
    }
}


pub struct HostDevice {
    compiler: Arc<Compiler>
}

impl HostDevice {
    pub fn new(compiler: &Arc<Compiler>) -> Self {
        HostDevice {
            compiler: compiler.clone()
        }
    }
}

impl Device for HostDevice {
    fn clean_app(&self, _build_bundle: &BuildBundle) -> Result<()> {
        unimplemented!()
    }

    fn debug_app(&self, _build_bundle: &BuildBundle, _args: &[&str], _envs: &[&str]) -> Result<()> {
        unimplemented!()
    }

    fn id(&self) -> &str {
        "HOST"
    }

    fn install_app(&self, _project: &Project, build: &Build, runnable: &Runnable) -> Result<BuildBundle> {
        debug!("No installation performed as it is not required for host platform");
        Ok(make_host_app(build, runnable)?)
    }

    fn name(&self) -> &str {
        "host device"
    }

    fn run_app(&self, _build_bundle: &BuildBundle, _args: &[&str], _envs: &[&str]) -> Result<()> {
        unimplemented!()
    }

    fn start_remote_lldb(&self) -> Result<String> {
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
