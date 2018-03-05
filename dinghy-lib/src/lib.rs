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
extern crate shell_escape;
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
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

use errors::*;

pub struct Dinghy {
    devices: Vec<Arc<Box<Device>>>,
    platforms: Vec<(String, Arc<Box<Platform>>)>,
}

impl Dinghy {
    pub fn probe(conf: &Arc<Configuration>, compiler: &Arc<Compiler>) -> Result<Dinghy> {
        let host = HostManager::probe(compiler).ok_or("Host platform couldn't be determined.")?;
        let mut managers: Vec<Box<PlatformManager>> = vec![Box::new(host)];

        if let Some(android) = AndroidManager::probe() {
            managers.push(Box::new(android))
        }
        if let Some(ssh) = SshDeviceManager::probe(conf.clone()) {
            managers.push(Box::new(ssh))
        }
        #[cfg(target_os = "macos")] {
            if let Some(m) = IosManager::new()? {
                managers.push(Box::new(m) as _)
            }
        }
        Ok(Dinghy {
            devices: Dinghy::discover_devices(&managers)?,
            platforms: Dinghy::discover_platforms(compiler, &conf)?,
        })
    }

    pub fn discover_platforms(compiler: &Arc<Compiler>, conf: &Configuration) -> Result<Vec<(String, Arc<Box<Platform>>)>> {
        let host_conf = conf.platforms.get("host")
            .map(|it| (*it).clone())
            .unwrap_or(PlatformConfiguration::empty());
        let mut platforms = vec![("host".to_string(),
                                  Arc::new(HostPlatform::new(compiler, host_conf)?))];

        for (platform_name, platform_conf) in conf.platforms.iter().skip(1) {
            if let Some(rustc_triple) = platform_conf.rustc_triple.as_ref() {
                let pf = if rustc_triple.ends_with("-ios") {
                    Dinghy::discover_ios_platform(platform_name.to_owned(), rustc_triple, compiler, platform_conf)?
                } else {
                    Some(RegularPlatform::new(
                        compiler,
                        (*platform_conf).clone(),
                        platform_name.to_string(),
                        rustc_triple.clone(),
                        platform_conf.toolchain.clone().ok_or(format!("Toolchain missing for platform {}", platform_name))?)?)
                };
                if let Some(pf) = pf {
                    platforms.push((platform_name.clone(), Arc::new(pf)))
                }
            } else {
                bail!("Platform configuration for '{}' requires a rustc_triple.", platform_name)
            }
        }

        Ok(platforms)
    }

    #[cfg(target_os = "macos")]
    fn discover_ios_platform(id: String, rustc_triple: &str, compiler: &Arc<Compiler>, config: &PlatformConfiguration) -> Result<Option<Box<Platform>>> {
        Ok(Some(IosPlatform::new(id, rustc_triple.clone(), compiler, config)?))
    }

    #[cfg(not(target_os = "macos"))]
    fn discover_ios_platform(id: String, rustc_triple: &str, _compiler: &Arc<Compiler>, _config: &PlatformConfiguration) -> Result<Option<Box<Platform>>> {
        warn!("Platform {} ({}) is an iOS one, and we are not on a Mac host.", id, rustc_triple);
        Ok(None)
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

    pub fn host_device(&self) -> Arc<Box<Device>> {
        self.devices[0].clone()
    }

    pub fn host_platform(&self) -> Arc<Box<Platform>> {
        self.platforms[0].1.clone()
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

pub trait Device: Display + DeviceCompatibility {
    fn clean_app(&self, build_bundle: &BuildBundle) -> Result<()>;

    fn debug_app(&self, project: &Project, build: &Build, args: &[&str], envs: &[&str]) -> Result<BuildBundle>;

    fn id(&self) -> &str;

    fn name(&self) -> &str;

    fn run_app(&self, project: &Project, build: &Build, args: &[&str], envs: &[&str]) -> Result<Vec<BuildBundle>>;

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

pub trait Platform {
    fn build(&self, project: &Project, build_args: &BuildArgs) -> Result<Build>;

    fn id(&self) -> String;

    fn is_compatible_with(&self, device: &Device) -> bool;

    fn rustc_triple(&self) -> Option<&str>;
}

pub trait PlatformManager {
    fn devices(&self) -> Result<Vec<Box<Device>>>;
}


#[derive(Clone, Debug)]
pub struct Build {
    pub build_args: BuildArgs,
    pub dynamic_libraries: Vec<PathBuf>,
    pub runnables: Vec<Runnable>,
    pub target_path: PathBuf,
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
