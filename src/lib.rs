extern crate cargo;
#[macro_use]
extern crate clap;
#[cfg(target_os = "macos")]
extern crate core_foundation;
#[cfg(target_os = "macos")]
extern crate core_foundation_sys;
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
use project::Project;
use std::fmt::Debug;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use errors::*;

pub trait PlatformManager {
    fn devices(&self) -> Result<Vec<Box<Device>>>;
}

pub trait DeviceCompatibility {
    fn is_compatible_with_regular_platform(&self, _platform: &regular_platform::RegularPlatform) -> bool {
        false
    }

    fn is_compatible_with_host_platform(&self, _platform: &host::HostPlatform) -> bool {
        false
    }

    #[cfg(target_os = "macos")]
    fn is_compatible_with_ios_platform(&self, _platform: &ios::IosToolchain) -> bool {
        false
    }
}

pub trait Device: Debug + Display + DeviceCompatibility {
    fn name(&self) -> &str;
    fn id(&self) -> &str;
    fn rustc_triple_guess(&self) -> Option<String>;
    fn can_run(&self, target: &str) -> bool {
        if let Some(t) = self.rustc_triple_guess() {
            t == target
        } else {
            true
        }
    }

    fn start_remote_lldb(&self) -> Result<String>;

    fn make_app(&self, project: &Project, source: &Path, app: &Path) -> Result<PathBuf>;
    fn install_app(&self, path: &Path) -> Result<()>;
    fn clean_app(&self, path: &Path) -> Result<()>;
    fn run_app(&self, app: &Path, args: &[&str], envs: &[&str]) -> Result<()>;
    fn debug_app(&self, app: &Path, args: &[&str], envs: &[&str]) -> Result<()>;
}

#[derive(Debug)]
pub struct Runnable {
    pub name: String,
    pub exe: PathBuf,
    pub source: PathBuf,
}

pub trait Platform: std::fmt::Debug {
    fn build(&self, compile_mode: CompileMode, matches: &clap::ArgMatches) -> Result<Vec<Runnable>>;

    fn id(&self) -> String;

    fn is_compatible_with(&self, device: &Device) -> bool;
}

pub struct Dinghy {
    managers: Vec<Box<PlatformManager>>,
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
        Ok(Dinghy { managers: managers })
    }

    #[cfg(not(target_os = "macos"))]
    fn new_ios_manager() -> Option<Box<PlatformManager>> {
        None
    }

    #[cfg(target_os = "macos")]
    fn new_ios_manager() -> Option<Box<ios::IosManager>> {
        ios::IosManager::new().unwrap_or(None).map(|it| Box::new(it) as Box<ios::IosManager>)
    }

    pub fn devices(&self) -> Result<Vec<Box<Device>>> {
        sleep(Duration::from_millis(100));
        let mut v = vec![];
        for m in &self.managers {
            v.extend(m.devices()?);
        }
        Ok(v)
    }
}
