extern crate cargo;
extern crate core_foundation;
extern crate core_foundation_sys;
#[macro_use]
extern crate error_chain;
extern crate libc;
#[macro_use]
extern crate log;
extern crate plist;
extern crate regex;
extern crate tempdir;

extern crate mobiledevice_sys;

pub mod ios;
pub mod android;
pub mod build;
pub mod errors;

use std::path;

use errors::*;

pub trait PlatformManager {
    fn devices(&self) -> Result<Vec<Box<Device>>>;
}

pub trait Device: std::fmt::Debug {
    fn name(&self) -> &str;
    fn id(&self) -> &str;
    fn target_arch(&self) -> &str;
    fn target_vendor(&self) -> &str;
    fn target_os(&self) -> &str;
    fn target(&self) -> String {
        format!("{}-{}-{}",
                self.target_arch(),
                self.target_vendor(),
                self.target_os())
    }
    fn start_remote_lldb(&self) -> Result<String>;

    fn make_app(&self, app: &path::Path, target:Option<&str>) -> Result<path::PathBuf>;
    fn install_app(&self, path: &path::Path) -> Result<()>;
    fn run_app(&self, app: &path::Path, args: &[&str]) -> Result<()>;
}

pub struct Dinghy {
    managers: Vec<Box<PlatformManager>>,
}

impl Default for Dinghy {
    fn default() -> Dinghy {
        let mut managers:Vec<Box<PlatformManager>> = vec![Box::new(ios::IosManager::default()) as Box<PlatformManager>];
        if let Some(android) = android::AndroidManager::probe() {
            managers.push(Box::new(android) as Box<PlatformManager>)
        }
        Dinghy {
            managers: managers
        }
    }
}

impl Dinghy {
    pub fn devices(&self) -> Result<Vec<Box<Device>>> {
        let mut v = vec!();
        for m in &self.managers {
            v.extend(m.devices()?);
        }
        Ok(v)
    }
}
