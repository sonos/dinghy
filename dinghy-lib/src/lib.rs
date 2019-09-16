#![type_length_limit = "2149570"]
extern crate atty;
extern crate cargo;
extern crate clap;
#[cfg(target_os = "macos")]
extern crate core_foundation;
#[cfg(target_os = "macos")]
extern crate core_foundation_sys;
extern crate dinghy_build;
extern crate dirs;
#[macro_use]
extern crate error_chain;
extern crate filetime;
extern crate ignore;
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
extern crate shell_escape;
#[cfg(target_os = "macos")]
extern crate tempdir;
extern crate toml;
extern crate walkdir;
extern crate which;

mod android;
pub mod compiler;
pub mod config;
pub mod device;
pub mod errors;
mod host;
#[cfg(target_os = "macos")]
mod ios;
pub mod overlay;
pub mod platform;
pub mod project;
mod script;
mod ssh;
mod toolchain;
pub mod utils;

pub use compiler::Compiler;
pub use config::Configuration;

use compiler::CompileMode;
use config::PlatformConfiguration;
#[cfg(target_os = "macos")]
use ios::IosManager;
use platform::regular_platform::RegularPlatform;
use project::Project;
use std::fmt::Display;
use std::{path, sync};

use errors::*;

pub struct Dinghy {
    devices: Vec<sync::Arc<Box<dyn Device>>>,
    platforms: Vec<(String, sync::Arc<Box<dyn Platform>>)>,
}

impl Dinghy {
    pub fn probe(
        conf: &sync::Arc<Configuration>,
        compiler: &sync::Arc<Compiler>,
    ) -> Result<Dinghy> {
        let mut managers: Vec<Box<dyn PlatformManager>> = vec![];
        if let Some(man) = host::HostManager::probe(sync::Arc::clone(compiler), conf) {
            managers.push(Box::new(man));
        }
        if let Some(man) = android::AndroidManager::probe(sync::Arc::clone(compiler)) {
            managers.push(Box::new(man));
        }
        if let Some(man) = script::ScriptDeviceManager::probe(conf.clone()) {
            managers.push(Box::new(man));
        }
        if let Some(man) = ssh::SshDeviceManager::probe(conf.clone()) {
            managers.push(Box::new(man));
        }
        #[cfg(target_os = "macos")]
        {
            thread::sleep(time::Duration::from_millis(100));
            if let Some(man) = IosManager::new(sync::Arc::clone(compiler))? {
                managers.push(Box::new(man));
            }
        }

        let mut devices = vec![];
        let mut platforms = vec![];
        for man in managers.into_iter() {
            devices.extend(man.devices()?.into_iter().map(|it| sync::Arc::new(it)));
            platforms.extend(
                man.platforms()?
                    .into_iter()
                    .map(|it| (it.id(), sync::Arc::new(it))),
            );
        }
        for (platform_name, platform_conf) in &conf.platforms {
            if platform_name == "host" {
                continue;
            }
            let rustc_triple = platform_conf
                .rustc_triple
                .as_ref()
                .ok_or_else(|| format!("Platform {} has no rustc_triple", platform_name))?;
            let pf = RegularPlatform::new(
                compiler,
                platform_conf.clone(),
                platform_name.to_string(),
                rustc_triple.clone(),
                platform_conf
                    .toolchain
                    .clone()
                    .map(|it| path::PathBuf::from(it))
                    .or(dirs::home_dir()
                        .map(|it| it.join(".dinghy").join("toolchain").join(platform_name)))
                    .ok_or(format!("Toolchain missing for platform {}", platform_name))?,
            )?;
            platforms.push((pf.id(), sync::Arc::new(pf)));
        }
        Ok(Dinghy { devices, platforms })
    }

    pub fn devices(&self) -> Vec<sync::Arc<Box<dyn Device>>> {
        self.devices.clone()
    }

    pub fn host_device(&self) -> sync::Arc<Box<dyn Device>> {
        self.devices[0].clone()
    }

    pub fn host_platform(&self) -> sync::Arc<Box<dyn Platform>> {
        self.platforms[0].1.clone()
    }

    pub fn platforms(&self) -> Vec<sync::Arc<Box<dyn Platform>>> {
        self.platforms
            .iter()
            .map(|&(_, ref platform)| platform.clone())
            .collect()
    }

    pub fn platform_by_name(
        &self,
        platform_name_filter: &str,
    ) -> Option<sync::Arc<Box<dyn Platform>>> {
        self.platforms
            .iter()
            .filter(|&&(ref platform_name, _)| platform_name == platform_name_filter)
            .map(|&(_, ref platform)| platform.clone())
            .next()
    }
}

pub trait Device: std::fmt::Debug + Display + DeviceCompatibility {
    fn clean_app(&self, build_bundle: &BuildBundle) -> Result<()>;

    fn debug_app(
        &self,
        project: &Project,
        build: &Build,
        args: &[&str],
        envs: &[&str],
    ) -> Result<BuildBundle>;

    fn id(&self) -> &str;

    fn name(&self) -> &str;

    fn run_app(
        &self,
        project: &Project,
        build: &Build,
        args: &[&str],
        envs: &[&str],
    ) -> Result<Vec<BuildBundle>>;

    fn start_remote_lldb(&self) -> Result<String>;
}

pub trait DeviceCompatibility {
    fn is_compatible_with_regular_platform(&self, _platform: &RegularPlatform) -> bool {
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

pub trait Platform: std::fmt::Debug {
    fn build(&self, project: &Project, build_args: &BuildArgs) -> Result<Build>;

    fn id(&self) -> String;

    fn is_compatible_with(&self, device: &dyn Device) -> bool;

    fn rustc_triple(&self) -> Option<&str>;

    fn strip(&self, build: &Build) -> Result<()>;
}

impl Display for dyn Platform {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "{}", self.id())
    }
}

pub trait PlatformManager {
    fn devices(&self) -> Result<Vec<Box<dyn Device>>>;
    fn platforms(&self) -> Result<Vec<Box<dyn Platform>>>;
}

#[derive(Clone, Debug)]
pub struct Build {
    pub build_args: BuildArgs,
    pub dynamic_libraries: Vec<path::PathBuf>,
    pub runnables: Vec<Runnable>,
    pub target_path: path::PathBuf,
}

#[derive(Clone, Debug)]
pub struct BuildArgs {
    pub compile_mode: CompileMode,
    pub verbose: bool,
    pub forced_overlays: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct BuildBundle {
    pub id: String,
    pub bundle_dir: path::PathBuf,
    pub bundle_exe: path::PathBuf,
    pub lib_dir: path::PathBuf,
    pub root_dir: path::PathBuf,
}

impl BuildBundle {
    fn replace_prefix_with<P: AsRef<path::Path>>(&self, path: P) -> Result<Self> {
        Ok(BuildBundle {
            id: self.id.clone(),
            bundle_dir: path
                .as_ref()
                .to_path_buf()
                .join(self.bundle_dir.strip_prefix(&self.root_dir)?),
            bundle_exe: path
                .as_ref()
                .to_path_buf()
                .join(self.bundle_exe.strip_prefix(&self.root_dir)?),
            lib_dir: path
                .as_ref()
                .to_path_buf()
                .join(self.lib_dir.strip_prefix(&self.root_dir)?),
            root_dir: path.as_ref().to_path_buf(),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Runnable {
    pub id: String,
    pub exe: path::PathBuf,
    pub source: path::PathBuf,
}
