#![type_length_limit = "2149570"]
#[cfg(target_os = "macos")]
extern crate tempdir;

pub mod errors {
    pub use anyhow::{anyhow, bail, Context, Error, Result};
}

mod android;
pub mod config;
pub mod device;
mod host;
#[cfg(target_os = "macos")]
mod apple;
pub mod overlay;
pub mod platform;
pub mod project;
mod script;
mod ssh;
mod toolchain;
pub mod utils;

pub use crate::config::Configuration;

use crate::config::PlatformConfiguration;
#[cfg(target_os = "macos")]
use crate::apple::{
    IosManager,
    TvosManager,
    WatchosManager,
};

use crate::platform::regular_platform::RegularPlatform;
use crate::project::Project;
use anyhow::{anyhow, Context};
use dyn_clone::DynClone;
use std::fmt::Display;
use std::{path, sync};

use crate::errors::Result;

pub struct Dinghy {
    devices: Vec<sync::Arc<Box<dyn Device>>>,
    platforms: Vec<(String, sync::Arc<Box<dyn Platform>>)>,
}

impl Dinghy {
    pub fn probe(conf: &sync::Arc<Configuration>) -> Result<Dinghy> {
        let mut managers: Vec<Box<dyn PlatformManager>> = vec![];
        if let Some(man) = host::HostManager::probe(conf) {
            managers.push(Box::new(man));
        }
        if let Some(man) = android::AndroidManager::probe() {
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
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Some(man) = IosManager::new().context("Could not initialize iOS manager")? {
                managers.push(Box::new(man));
            }
            if let Some(man) = TvosManager::new().context("Could not initialize tvOS manager")? {
                managers.push(Box::new(man));
            }
            if let Some(man) = WatchosManager::new().context("Could not initialize tvOS manager")? {
                managers.push(Box::new(man));
            }
        }

        let mut devices = vec![];
        let mut platforms = vec![];
        for man in managers.into_iter() {
            devices.extend(
                man.devices()
                    .context("Could not list devices")?
                    .into_iter()
                    .map(|it| sync::Arc::new(it)),
            );
            platforms.extend(
                man.platforms()
                    .context("Could not list platforms")?
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
                .ok_or_else(|| anyhow!("Platform {} has no rustc_triple", platform_name))?;
            let pf = RegularPlatform::new(
                platform_conf.clone(),
                platform_name.to_string(),
                rustc_triple.clone(),
                platform_conf
                    .toolchain
                    .clone()
                    .map(|it| path::PathBuf::from(it))
                    .or(dirs::home_dir()
                        .map(|it| it.join(".dinghy").join("toolchain").join(platform_name)))
                    .with_context(|| format!("Toolchain missing for platform {}", platform_name))?,
            )
            .with_context(|| format!("Could not assemble platform {}", platform_name))?;
            platforms.push((pf.id(), sync::Arc::new(pf)));
        }
        Ok(Dinghy { devices, platforms })
    }

    pub fn devices(&self) -> Vec<sync::Arc<Box<dyn Device>>> {
        self.devices.clone()
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

pub trait Device: std::fmt::Debug + Display + DeviceCompatibility + DynClone {
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
    ) -> Result<BuildBundle>;
}

dyn_clone::clone_trait_object!(Device);

pub trait DeviceCompatibility {
    fn is_compatible_with_regular_platform(&self, _platform: &RegularPlatform) -> bool {
        false
    }

    fn is_compatible_with_host_platform(&self, _platform: &host::HostPlatform) -> bool {
        false
    }

    #[cfg(target_os = "macos")]
    fn is_compatible_with_simulator_platform(&self, _platform: &apple::AppleDevicePlatform) -> bool {
        false
    }
}

pub trait Platform: std::fmt::Debug {
    fn setup_env(&self, project: &Project, setup_args: &SetupArgs) -> Result<()>;

    fn id(&self) -> String;

    fn is_compatible_with(&self, device: &dyn Device) -> bool;

    fn is_host(&self) -> bool;
    fn rustc_triple(&self) -> &str;

    fn strip(&self, build: &mut Build) -> Result<()>;
    fn sysroot(&self) -> Result<Option<path::PathBuf>>;
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
    pub setup_args: SetupArgs,
    pub dynamic_libraries: Vec<path::PathBuf>,
    pub runnable: Runnable,
    pub target_path: path::PathBuf,
    pub files_in_run_args: Vec<path::PathBuf>,
}

#[derive(Clone, Debug)]
pub struct SetupArgs {
    pub verbosity: i8,
    pub forced_overlays: Vec<String>,
    pub envs: Vec<String>,
    pub cleanup: bool,
    pub strip: bool,
    pub device_id: Option<String>,
}

impl SetupArgs {
    pub fn get_runner_command(&self, platform_id: &str) -> String {
        let mut extra_args = String::new();
        if self.verbosity > 0 {
            for _ in 0..self.verbosity {
                extra_args.push_str("-v ")
            }
        }
        if self.verbosity < 0 {
            for _ in 0..-self.verbosity {
                extra_args.push_str("-q ")
            }
        }
        if self.cleanup {
            extra_args.push_str("--cleanup ")
        }
        if self.strip {
            extra_args.push_str("--strip ")
        }
        if let Some(device_id) = &self.device_id {
            extra_args.push_str("-d ");
            extra_args.push_str(&device_id);
            extra_args.push(' ');
        }
        for env in &self.envs {
            extra_args.push_str("-e ");
            extra_args.push_str(env);
            extra_args.push(' ');
        }

        format!(
            "{} -p {} {}runner --",
            std::env::current_exe().unwrap().to_str().unwrap(),
            platform_id,
            extra_args
        )
    }
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
            bundle_dir: utils::normalize_path(
                &path
                    .as_ref()
                    .to_path_buf()
                    .join(self.bundle_dir.strip_prefix(&self.root_dir)?),
            ),
            bundle_exe: utils::normalize_path(
                &path
                    .as_ref()
                    .to_path_buf()
                    .join(self.bundle_exe.strip_prefix(&self.root_dir)?),
            ),
            lib_dir: utils::normalize_path(
                &path
                    .as_ref()
                    .to_path_buf()
                    .join(self.lib_dir.strip_prefix(&self.root_dir)?),
            ),
            root_dir: path.as_ref().to_path_buf(),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct Runnable {
    pub id: String,
    pub package_name: String,
    pub exe: path::PathBuf,
    pub source: path::PathBuf,
}
