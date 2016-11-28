extern crate cargo;
#[cfg(target_os="macos")]
extern crate core_foundation;
#[cfg(target_os="macos")]
extern crate core_foundation_sys;
#[macro_use]
extern crate error_chain;
extern crate libc;
#[macro_use]
extern crate log;
extern crate plist;
extern crate regex;
extern crate tempdir;


#[cfg(target_os="macos")]
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

    fn make_app(&self, app: &path::Path) -> Result<path::PathBuf>;
    fn install_app(&self, path: &path::Path) -> Result<()>;
    fn run_app(&self, app: &path::Path, args: &[&str]) -> Result<()>;
}

pub struct Dinghy {
    managers: Vec<Box<PlatformManager>>,
}

impl Dinghy {
    pub fn probe() -> Result<Dinghy> {
        let mut managers:Vec<Box<PlatformManager>> = vec![];
        if let Some(ios) = ios::IosManager::new()? {
            managers.push(Box::new(ios) as Box<PlatformManager>)
        }
        if let Some(android) = android::AndroidManager::probe() {
            managers.push(Box::new(android) as Box<PlatformManager>)
        }
        Ok(Dinghy {
            managers: managers
        })
    }

    pub fn devices(&self) -> Result<Vec<Box<Device>>> {
        let mut v = vec!();
        for m in &self.managers {
            v.extend(m.devices()?);
        }
        Ok(v)
    }
}

#[cfg(not(target_os="macos"))]
pub mod ios {
    pub struct IosManager{}
    pub struct IosDevice{}
    impl ::PlatformManager for IosManager {
        fn devices(&self) -> ::errors::Result<Vec<Box<::Device>>> {
            Ok(vec!())
        }
    }
    impl IosManager {
        pub fn new() -> ::errors::Result<Option<IosManager>> {
            Ok((None))
        }
    }
}
