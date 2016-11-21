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
pub mod xcode;
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
    fn install_app(&self, path:&path::Path) -> Result<()>;
    fn run_app(&self, app:&path::Path, app_id:&str) -> Result<()>;
}

pub struct Dinghy {
    ios: Option<Box<PlatformManager>>,
}

impl Default for Dinghy {
    fn default() -> Dinghy {
        Dinghy { ios: Some(Box::new(ios::IosManager::default())) }
    }
}

impl Dinghy {
    pub fn devices(&self) -> Result<Vec<Box<Device>>> {
        self.ios.as_ref().unwrap().devices()
    }
}
