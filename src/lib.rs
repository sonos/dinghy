extern crate cargo;
#[cfg(target_os="macos")]
extern crate core_foundation;
#[cfg(target_os="macos")]
extern crate core_foundation_sys;
#[macro_use]
extern crate error_chain;
extern crate ignore;
extern crate json;
#[cfg(target_os="macos")]
extern crate libc;
#[macro_use]
extern crate log;
extern crate plist;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[cfg(target_os="macos")]
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

    fn make_app(&self, source: &path::Path, app: &path::Path) -> Result<path::PathBuf>;
    fn install_app(&self, path: &path::Path) -> Result<()>;
    fn clean_app(&self, path: &path::Path) -> Result<()>;
    fn run_app(&self, app: &path::Path, args: &[&str], envs: &[&str]) -> Result<()>;
    fn debug_app(&self, app: &path::Path, args: &[&str], envs: &[&str]) -> Result<()>;
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

fn make_linux_app(root: &path::Path, exe: &path::Path) -> Result<path::PathBuf> {
    let app_name = "dinghy";
    let app_path = exe.parent().unwrap().join("dinghy").join(app_name);
    debug!("Making bundle {:?} for {:?}", app_path, exe);
    fs::create_dir_all(app_path.join("src"))?;
    fs::copy(&exe, app_path.join(app_name))?;
    debug!("Copying src to bundle");
    ::rec_copy(root, app_path.join("src"), false)?;
    debug!("Copying test_data to bundle");
    ::copy_test_data(root, &app_path)?;
    Ok(app_path.into())
}

fn copy_test_data<S: AsRef<path::Path>, T: AsRef<path::Path>>(root: S, app_path: T) -> Result<()> {
    let app_path = app_path.as_ref();
    fs::create_dir_all(app_path.join("test_data"))?;
    let conf = config::config(root.as_ref())?;
    for td in conf.test_data {
        let root = path::PathBuf::from("/");
        let file = td.base.parent().unwrap_or(&root).join(&td.source);
        if path::Path::new(&file).exists() {
            let metadata = file.metadata()?;
            let dst = app_path.join("test_data").join(td.target);
            if metadata.is_dir() {
                ::rec_copy(file, dst, td.copy_git_ignored)?;
            } else {
                fs::copy(file
                         , dst)?;
            }
        } else {
            warn!("configuration required test_data `{:?}` but it could not be found", td);
        }
    }
    Ok(())
}

fn rec_copy<P1: AsRef<path::Path>,P2: AsRef<path::Path>>(src:P1, dst:P2, copy_ignored_test_data: bool) -> Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    let ignore_file = src.join(".dinghyignore");
    fs::create_dir_all(&dst)?;
    let mut walker = ignore::WalkBuilder::new(src);
    walker.git_ignore(!copy_ignored_test_data);
    walker.add_ignore(ignore_file);
    for entry in walker.build() {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let path = entry.path().strip_prefix(src)?;
        if path.components().any(|comp| comp == std::path::Component::Normal("target".as_ref()) ) {
            continue;
        }
        let target = dst.join(path);
        if metadata.is_dir() {
            if target.exists() && !target.is_dir() {
                fs::remove_dir_all(&target)?;
            }&
            fs::create_dir_all(&target)?;
        } else {
            if target.exists() && !target.is_file() {
                fs::remove_dir_all(&target)?;
            }
            if !target.exists()
                || target.metadata()?.len() != entry.metadata()?.len()
                || target.metadata()?.modified()? < entry.metadata()?.modified()? {
                fs::copy(entry.path(), &target)?;
            }
        }
    }
    Ok(())
}
