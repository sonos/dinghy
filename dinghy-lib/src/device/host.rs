use compiler::Compiler;
use compiler::CompileMode;
use overlay::Overlayer;
use platform::host::HostPlatform;
use project::Project;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::path::Path;
use Build;
use BuildBundle;
use Device;
use Platform;
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

    fn platform(&self) -> Result<Box<Platform>> {
        Ok(HostPlatform::new()?)
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

impl Platform for HostPlatform {
    fn build(&self, compiler: &Compiler, compile_mode: CompileMode) -> Result<Build> {
        Overlayer::new(self, "/", compiler.target_dir(self.rustc_triple())?.join(&self.id))
            .overlay(&self.configuration, compiler.project_dir()?)?;

        compiler.build(self, compile_mode)
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

    fn is_system_path(&self, path: &Path) -> Result<bool> {
        let ignored_path = vec![
            Path::new("/lib"),
            Path::new("/usr/lib"),
            Path::new("/usr/lib32"),
            Path::new("/usr/lib64"),
        ];
        Ok(!ignored_path.iter().any(|it| path.starts_with(it)))
    }
}