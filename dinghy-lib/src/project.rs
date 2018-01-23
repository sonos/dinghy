use config::dinghy_config;
use config::Configuration;
use ignore::WalkBuilder;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
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

    pub fn for_runnable(&self, runnable: &Runnable) -> Result<Self> {
        Ok(Project {
            conf: Arc::new(dinghy_config(&runnable.source)?),
        })
    }

    pub fn copy_test_data<T: AsRef<Path>>(&self, app_path: T) -> Result<()> {
        let app_path = app_path.as_ref();
        fs::create_dir_all(app_path)?;
        for td in self.conf.test_data.iter() {
            let root = PathBuf::from("/");
            let file = td.base.parent().unwrap_or(&root).join(&td.source);
            if Path::new(&file).exists() {
                let dst = app_path.join(&td.target);
                self.rec_copy(file, dst, td.copy_git_ignored)?;
            } else {
                warn!("Configuration required test_data {:?} but it could not be found", td);
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

        let mut walker = WalkBuilder::new(src);
        walker.git_ignore(!copy_ignored_test_data);
        walker.add_ignore(ignore_file);
        for entry in walker.build() {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let path = entry.path().strip_prefix(src)?;

            // Check if root path is a file or a directory
            let target =  if path.parent().is_none() && metadata.is_file() {
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
                    fs::copy(entry.path(), &target)?;
                } else {
                    debug!("{} is already up-to-date", target.display());
                }
            }
        }
        Ok(())
    }
}