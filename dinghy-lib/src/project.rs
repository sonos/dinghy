use config::dinghy_config;
use config::Configuration;
use filetime::set_file_times;
use filetime::FileTime;
use ignore::WalkBuilder;
use std::fs;
use std::fs::File;
use std::env::current_dir;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use Platform;
use Result;
use Runnable;

#[derive(Debug)]
pub struct Project {
    pub conf: Arc<Configuration>
}

impl Project {
    pub fn new(conf: &Arc<Configuration>) -> Project {
        Project {
            conf: conf.clone(),
        }
    }

    pub fn project_dir(&self) -> Result<PathBuf> {
        let wd_path = ::cargo::util::important_paths::find_root_manifest_for_wd(None, &current_dir()?)?;
        Ok(wd_path.parent()
            .ok_or(format!("Couldn't read project directory {}.", wd_path.display()))?
            .to_path_buf())
    }

    pub fn overlay_work_dir(&self, platform: &Platform) -> Result<PathBuf> {
        Ok(self
            .target_dir(platform.rustc_triple())?
            .join(platform.id()))
    }

    pub fn target_dir(&self, rustc_triple: Option<&str>) -> Result<PathBuf> {
        let mut target_path = self.project_dir()?.join("target");
        if let Some(rustc_triple) = rustc_triple {
            target_path = target_path.join(rustc_triple);
        }
        Ok(target_path)
    }

    pub fn for_runnable(&self, runnable: &Runnable) -> Result<Self> {
        Ok(Project {
            conf: Arc::new(dinghy_config(&runnable.source)?),
        })
    }

    pub fn link_test_data<T: AsRef<Path>>(&self, runnable: &Runnable, app_path: T) -> Result<()> {
        let app_path = app_path.as_ref();
        let sub_project = self.for_runnable(runnable)?;
        let test_data_path = app_path.join("test_data");
        fs::create_dir_all(&test_data_path)?;
        let test_data_cfg_path = test_data_path.join("test_data.cfg");
        let mut test_data_cfg = File::create(&test_data_cfg_path)?;
        debug!("Generating {}", test_data_cfg_path.display());

        for td in sub_project.conf.test_data.iter() {
            let target_path = td.base.parent().unwrap_or(&PathBuf::from("/")).join(&td.source);
            let target_path = target_path.to_str().ok_or(format!("Invalid UTF-8 path {}", target_path.display()))?;

            test_data_cfg.write_all(td.id.as_bytes())?;
            test_data_cfg.write_all(b":")?;
            test_data_cfg.write_all(target_path.as_bytes())?;
            test_data_cfg.write_all(b"\n")?;
        }
        Ok(())
    }

    pub fn copy_test_data<T: AsRef<Path>>(&self, app_path: T) -> Result<()> {
        let app_path = app_path.as_ref();
        let test_data_path = app_path.join("test_data");
        fs::create_dir_all(&test_data_path)?;

        for td in self.conf.test_data.iter() {
            let file = td.base.parent().unwrap_or(&PathBuf::from("/")).join(&td.source);
            if Path::new(&file).exists() {
                let metadata = file.metadata()?;
                let dst = test_data_path.join(&td.id);
                if metadata.is_dir() {
                    self.rec_copy(file, dst, td.copy_git_ignored)?;
                } else {
                    fs::copy(file, dst)?;
                }
            } else {
                warn!("configuration required test_data `{:?}` but it could not be found", td);
            }
        }
        Ok(())
    }

    pub fn rec_copy<P1: AsRef<Path>, P2: AsRef<Path>>(
        &self,
        src: P1,
        dst: P2,
        copy_ignored_test_data: bool,
    ) -> Result<()> {
        let src = src.as_ref();
        let dst = dst.as_ref();
        let ignore_file = src.join(".dinghyignore");
        debug!("Copying recursively from {} to {}", src.display(), dst.display());

        let mut walker = WalkBuilder::new(src);
        walker.git_ignore(!copy_ignored_test_data);
        walker.add_ignore(ignore_file);
        for entry in walker.build() {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let path = entry.path().strip_prefix(src)?;

            // Check if root path is a file or a directory
            let target = if path.parent().is_none() && metadata.is_file() {
                fs::create_dir_all(&dst.parent()
                    .ok_or(format!("Invalid file {}", dst.display()))?)?;
                dst.to_path_buf()
            } else {
                dst.join(path)
            };

            if metadata.is_dir() {
                if target.exists() && !target.is_dir() {
                    fs::remove_dir_all(&target)?;
                }
                debug!("Creating directory {}", target.display());
                &fs::create_dir_all(&target)?;
            } else {
                if target.exists() && !target.is_file() {
                    fs::remove_dir_all(&target)?;
                }
                if !target.exists()
                    || target.metadata()?.len() != entry.metadata()?.len()
                    || target.metadata()?.modified()? < entry.metadata()?.modified()? {
                    if target.exists() && target.metadata()?.permissions().readonly() {
                        fs::remove_dir_all(&target)?;
                    }
                    debug!("Copying {} to {}", entry.path().display(), target.display());
                    copy_file(entry.path(), &target)?;
                } else {
                    debug!("{} is already up-to-date", target.display());
                }
            }
        }
        Ok(())
    }
}

pub fn copy_file<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<()> {
    let from = &from.as_ref();
    let to = &to.as_ref();
    fs::copy(&from, &to)?;

    // Keep filetime to avoid useless sync on some devices (e.g. Android).
    let from_metadata = from.metadata()?;
    let atime = FileTime::from_last_access_time(&from_metadata);
    let mtime = FileTime::from_last_modification_time(&from_metadata);
    set_file_times(&to, atime, mtime)?;

    Ok(())
}
