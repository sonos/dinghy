extern crate cargo;
#[cfg(target_os="macos")]
extern crate core_foundation;
#[cfg(target_os="macos")]
extern crate core_foundation_sys;
#[macro_use]
extern crate error_chain;
extern crate ignore;
extern crate json;
extern crate libc;
#[macro_use]
extern crate log;
extern crate plist;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate tempdir;
extern crate toml;

#[cfg(target_os="macos")]
pub mod ios;
pub mod config;

pub mod android;
pub mod ssh;
pub mod errors;

mod linker;
pub use linker::setup_linker;

use std::{ fs, path };

use errors::*;

pub trait PlatformManager {
    fn devices(&self) -> Result<Vec<Box<Device>>>;
}

pub trait Device: std::fmt::Debug {
    fn name(&self) -> &str;
    fn id(&self) -> &str;
    fn target(&self) -> String;
    fn can_run(&self, target:&str) -> bool {
        target == self.target()
    }
    fn start_remote_lldb(&self) -> Result<String>;

    fn make_app(&self, app: &path::Path) -> Result<path::PathBuf>;
    fn install_app(&self, path: &path::Path) -> Result<()>;
    fn run_app(&self, app: &path::Path, args: &[&str]) -> Result<()>;
    fn debug_app(&self, app: &path::Path, args: &[&str]) -> Result<()>;
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
        if let Some(config) = ssh::SshDeviceManager::probe() {
            managers.push(Box::new(config) as Box<PlatformManager>)
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

fn make_linux_app(exe: &path::Path) -> Result<path::PathBuf> {
    let app_name = exe.file_name().unwrap();
    let app_path = exe.parent().unwrap().join("dinghy").join(app_name);
    debug!("Making bundle {:?} for {:?}", app_path, exe);
    let _ = fs::remove_dir_all(&app_path);
    fs::create_dir_all(app_path.join("src"))?;
    fs::copy(&exe, app_path.join(app_name))?;
    debug!("Copying src to bundle");
    ::rec_copy(".", app_path.join("src"))?;
    debug!("Copying test_data to bundle");
    ::copy_test_data(&app_path)?;
    Ok(app_path.into())
}

fn copy_test_data<P: AsRef<path::Path>>(app_path: P) -> Result<()> {
    let app_path = app_path.as_ref();
    fs::create_dir_all(app_path.join("test_data"))?;
    let conf = config::config()?;
    for (k,v) in conf.test_data {
        if path::Path::new(&v).exists() {
            let metadata = path::Path::new(&v).metadata()?;
            let dst = app_path.join("test_data").join(k);
            if metadata.is_dir() {
                ::rec_copy(v, dst)?;
            } else {
                fs::copy(v, dst)?;
            }
        } else {
            warn!("configuration required test_data `{}` from `{}` but it could not be found", k, v);
        }
    }
    Ok(())
}

fn rec_copy<P1: AsRef<path::Path>,P2: AsRef<path::Path>>(src:P1, dst:P2) -> Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    fs::create_dir_all(&dst)?;
    for entry in ignore::WalkBuilder::new(src).build() {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            fs::create_dir_all(dst.join(entry.path().strip_prefix(src)?))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.path().strip_prefix(src)?))?;
        }
    }
    Ok(())
}
