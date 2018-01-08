extern crate cargo;
#[macro_use]
extern crate clap;
#[cfg(target_os = "macos")]
extern crate core_foundation;
#[cfg(target_os = "macos")]
extern crate core_foundation_sys;
extern crate dinghy_helper;
#[macro_use]
extern crate error_chain;
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

pub mod android;
pub mod cli;
pub mod cargo_facade;
pub mod config;
pub mod errors;
pub mod host;
#[cfg(target_os = "macos")]
pub mod ios;
pub mod project;
pub mod regular_platform;
pub mod ssh;
mod toolchain;

use cargo_facade::CompileMode;
use config::Configuration;
use config::PlatformConfiguration;
use host::HostPlatform;
#[cfg(target_os = "macos")]
use ios::IosPlatform;
use project::Project;
use regular_platform::RegularPlatform;
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
        if let Some(host) = host::HostManager::probe() {
            managers.push(Box::new(host))
        }
        if let Some(android) = android::AndroidManager::probe() {
            managers.push(Box::new(android))
        }
        if let Some(ssh) = ssh::SshDeviceManager::probe(conf.clone()) {
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
    fn new_ios_manager() -> Option<Box<ios::IosManager>> {
        ios::IosManager::new().unwrap_or(None).map(|it| Box::new(it) as Box<ios::IosManager>)
    }

    pub fn discover_platforms(conf: &Configuration) -> Result<Vec<(String, Arc<Box<Platform>>)>> {
        conf.platforms
            .iter()
            .filter(Dinghy::unavailable_platforms)
            .map(|(platform_name, platform_conf)| {
                if let Some(rustc_triple) = platform_conf.rustc_triple.as_ref() {
                    if rustc_triple.ends_with("-ios") {
                        Dinghy::discover_ios_platform(rustc_triple)
                    } else {
                        RegularPlatform::new(
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
    fn unavailable_platforms(&(_platform_name, _platform_conf): &(&String, &PlatformConfiguration)) -> bool {
        false
    }

    #[cfg(target_os = "macos")]
    fn discover_ios_platform(rustc_triple: &str) -> Result<Box<Platform>> {
        Some(Arc::new(IosPlatform::new(rustc_triple.clone())))
    }

    #[cfg(not(target_os = "macos"))]
    fn unavailable_platforms(&(_platform_name, platform_conf): &(&String, &PlatformConfiguration)) -> bool {
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
    fn clean_app(&self, path: &Path) -> Result<()>;
    fn debug_app(&self, app: &Path, args: &[&str], envs: &[&str]) -> Result<()>;
    fn id(&self) -> &str;
    fn install_app(&self, path: &Path) -> Result<()>;
    fn make_app(&self, project: &Project, source: &Path, app: &Path) -> Result<PathBuf>;
    fn name(&self) -> &str;
    fn platform(&self) -> Result<Box<Platform>>;
    fn run_app(&self, app: &Path, args: &[&str], envs: &[&str]) -> Result<()>;
    fn start_remote_lldb(&self) -> Result<String>;
}

pub trait DeviceCompatibility {
    fn is_compatible_with_regular_platform(&self, _platform: &regular_platform::RegularPlatform) -> bool {
        false
    }

    fn is_compatible_with_host_platform(&self, _platform: &host::HostPlatform) -> bool {
        false
    }

    #[cfg(target_os = "macos")]
    fn is_compatible_with_ios_platform(&self, _platform: &ios::IosPlatform) -> bool {
        false
    }
}

pub trait Platform: Debug {
    fn build(&self, compile_mode: CompileMode, matches: &clap::ArgMatches) -> Result<Vec<Runnable>>;

    fn id(&self) -> String;

    fn is_compatible_with(&self, device: &Device) -> bool;
}

pub trait PlatformManager {
    fn devices(&self) -> Result<Vec<Box<Device>>>;
}

#[derive(Debug)]
pub struct Runnable {
    pub name: String,
    pub exe: PathBuf,
    pub source: PathBuf,
}
