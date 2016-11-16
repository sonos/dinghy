extern crate core_foundation;
extern crate core_foundation_sys;
#[macro_use]
extern crate error_chain;
extern crate libc;
extern crate tempdir;

extern crate mobiledevice_sys;

mod ios;
pub mod errors;
use errors::*;

pub trait PlatformManager {
    fn devices(&self) -> Result<Vec<Box<Device>>>;
}

pub trait Device : std::fmt::Debug {
    fn name(&self) -> &str;
    fn target_arch(&self) -> &str;
    fn target_vendor(&self) -> &str;
    fn target_os(&self) -> &str;
    fn target(&self) -> String {
        format!("{}-{}-{}", self.target_arch(), self.target_vendor(), self.target_os())
    }
}

pub struct Dinghy {
    ios: Option<Box<PlatformManager>>,
}

impl Default for Dinghy {
    fn default() -> Dinghy {
        Dinghy {
            ios: Some(Box::new(ios::IosManager::default()))
        }
    }
}

impl Dinghy {
    pub fn devices(&self) -> Result<Vec<Box<Device>>> {
        self.ios.as_ref().unwrap().devices()
    }
}
