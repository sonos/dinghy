extern crate cargo;
extern crate clap;
#[cfg(target_os = "macos")]
extern crate core_foundation;
#[cfg(target_os = "macos")]
extern crate core_foundation_sys;
extern crate dinghy_helper;
#[macro_use]
extern crate error_chain;
extern crate filetime;
extern crate ignore;
extern crate isatty;
extern crate itertools;
extern crate json;
#[cfg(target_os = "macos")]
extern crate libc;
#[macro_use]
extern crate log;
extern crate plist;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[cfg(target_os = "macos")]
extern crate tempdir;
extern crate toml;
extern crate walkdir;

pub mod compiler;
pub mod config;
pub mod device;
pub mod errors;
pub mod overlay;
pub mod platform;
pub mod project;
pub mod utils;
mod toolchain;

use compiler::Compiler;
use compiler::CompileMode;
use config::Configuration;
use config::PlatformConfiguration;
use device::android::AndroidManager;
use device::host::HostManager;
#[cfg(target_os = "macos")]
use device::ios::IosManager;
use device::ssh::SshDeviceManager;
use platform::host::HostPlatform;
#[cfg(target_os = "macos")]
use platform::ios::IosPlatform;
use platform::regular_platform::RegularPlatform;
use project::Project;
use std::fmt::Debug;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use errors::*;

pub struct Dinghy {
    platforms: Vec<(String, Arc<Box<Platform>>)>,
    devices: Vec<Arc<Box<Device>>>,
}

impl Dinghy {
    pub fn probe(conf: &Arc<Configuration>) -> Result<Dinghy> {
        let mut managers: Vec<Box<PlatformManager>> = vec![];
        if let Some(host) = HostManager::probe() {
            managers.push(Box::new(host))
        }
        if let Some(android) = AndroidManager::probe() {
            managers.push(Box::new(android))
        }
        if let Some(ssh) = SshDeviceManager::probe(conf.clone()) {
            managers.push(Box::new(ssh))
        }
        if let Some(ios) = Dinghy::new_ios_manager() {
            managers.push(ios)
        }

        Ok(Dinghy {
            platforms: Dinghy::discover_platforms(&conf)?,
            devices: Dinghy::discover_devices(&managers)?,
        })
    }

    #[cfg(not(target_os = "macos"))]
    fn new_ios_manager() -> Option<Box<PlatformManager>> {
        None
    }

    #[cfg(target_os = "macos")]
    fn new_ios_manager() -> Option<Box<IosManager>> {
        IosManager::new().unwrap_or(None).map(|it| Box::new(it) as Box<IosManager>)
    }

    pub fn discover_platforms(conf: &Configuration) -> Result<Vec<(String, Arc<Box<Platform>>)>> {
        conf.platforms
            .iter()
            .filter(Dinghy::available_platforms)
            .map(|(platform_name, platform_conf)| {
                if let Some(rustc_triple) = platform_conf.rustc_triple.as_ref() {
                    if rustc_triple.ends_with("-ios") {
                        Dinghy::discover_ios_platform(rustc_triple)
                    } else {
                        RegularPlatform::new(
                            (*platform_conf).clone(),
                            platform_name.to_string(),
                            rustc_triple.clone(),
                            platform_conf.toolchain.clone().ok_or(format!("Toolchain missing for platform {}", platform_name))?)
                    }
                } else {
                    HostPlatform::new()
                }
                    .map(|platform| (platform_name.clone(), Arc::new(platform)))
            })
            .collect()
    }

    #[cfg(target_os = "macos")]
    fn available_platforms(&(_platform_name, _platform_conf): &(&String, &PlatformConfiguration)) -> bool {
        true
    }

    #[cfg(target_os = "macos")]
    fn discover_ios_platform(rustc_triple: &str) -> Result<Box<Platform>> {
        Ok(IosPlatform::new(rustc_triple.clone())?)
    }

    #[cfg(not(target_os = "macos"))]
    fn available_platforms(&(_platform_name, platform_conf): &(&String, &PlatformConfiguration)) -> bool {
        platform_conf.rustc_triple.as_ref().map(|it| !it.ends_with("-ios")).unwrap_or(true)
    }

    #[cfg(not(target_os = "macos"))]
    fn discover_ios_platform(_rustc_triple: &str) -> Result<Box<Platform>> {
        unimplemented!()
    }

    fn discover_devices(managers: &Vec<Box<PlatformManager>>) -> Result<Vec<Arc<Box<Device>>>> {
        sleep(Duration::from_millis(100));
        let mut v = vec![];
        for m in managers {
            v.extend(m.devices()?.into_iter().map(|it| Arc::new(it)));
        }
        Ok(v)
    }

    pub fn devices(&self) -> Vec<Arc<Box<Device>>> {
        self.devices.clone()
    }

    pub fn platforms(&self) -> Vec<Arc<Box<Platform>>> {
        self.platforms.iter()
            .map(|&(_, ref platform)| platform.clone())
            .collect()
    }

    pub fn platform_by_name(&self, platform_name_filter: &str) -> Option<Arc<Box<Platform>>> {
        self.platforms.iter()
            .filter(|&&(ref platform_name, _)| platform_name == platform_name_filter)
            .map(|&(_, ref platform)| platform.clone())
            .next()
    }
}

pub trait Device: Debug + Display + DeviceCompatibility {
    fn clean_app(&self, build_bundle: &BuildBundle) -> Result<()>;

    fn debug_app(&self, build_bundle: &BuildBundle, args: &[&str], envs: &[&str]) -> Result<()>;

    fn id(&self) -> &str;

    fn install_app(&self, project: &Project, build: &Build, runnable: &Runnable) -> Result<BuildBundle>;

    fn name(&self) -> &str;

    fn run_app(&self, build_bundle: &BuildBundle, args: &[&str], envs: &[&str]) -> Result<()>;

    fn start_remote_lldb(&self) -> Result<String>;
}

pub trait DeviceCompatibility {
    fn is_compatible_with_regular_platform(&self, _platform: &RegularPlatform) -> bool {
        false
    }

    fn is_compatible_with_host_platform(&self, _platform: &HostPlatform) -> bool {
        false
    }

    #[cfg(target_os = "macos")]
    fn is_compatible_with_ios_platform(&self, _platform: &IosPlatform) -> bool {
        false
    }
}

pub trait Platform: Debug {
    fn build(&self, compiler: &Compiler, compile_mode: CompileMode) -> Result<Build>;

    fn id(&self) -> String;

    fn is_compatible_with(&self, device: &Device) -> bool;

    fn rustc_triple(&self) -> Option<&str>;
}

pub trait PlatformManager {
    fn devices(&self) -> Result<Vec<Box<Device>>>;
}

#[derive(Clone, Debug, Default)]
pub struct Build {
    pub dynamic_libraries: Vec<PathBuf>,
    pub runnables: Vec<Runnable>,
    pub target_path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct BuildBundle {
    pub id: String,
    pub bundle_dir: PathBuf,
    pub bundle_exe: PathBuf,
    pub lib_dir: PathBuf,
    pub root_dir: PathBuf,
}

impl BuildBundle {
    fn replace_prefix_with<P: AsRef<Path>>(&self, path: P) -> Result<Self> {
        Ok(BuildBundle {
            id: self.id.clone(),
            bundle_dir: path.as_ref().to_path_buf()
                .join(self.bundle_dir.strip_prefix(&self.root_dir)?),
            bundle_exe: path.as_ref().to_path_buf()
                .join(self.bundle_exe.strip_prefix(&self.root_dir)?),
            lib_dir: path.as_ref().to_path_buf()
                .join(self.lib_dir.strip_prefix(&self.root_dir)?),
            root_dir: path.as_ref().to_path_buf(),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Runnable {
    pub id: String,
    pub exe: PathBuf,
    pub source: PathBuf,
}
