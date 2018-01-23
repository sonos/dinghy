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
        fs::create_dir_all(app_path.join("test_data"))?;
        for td in self.conf.test_data.iter() {
            let root = PathBuf::from("/");
            let file = td.base.parent().unwrap_or(&root).join(&td.source);
            if Path::new(&file).exists() {
                let metadata = file.metadata()?;
                let dst = app_path.join("test_data").join(&td.target);
                debug!("Copying test data '{}' to '{}'", file.display(), dst.display());
                if metadata.is_dir() {
                    self.rec_copy(file, dst, td.copy_git_ignored)?;
                } else {
                    fs::copy(file, dst)?;
                }
            } else {
                warn!("Configuration required test_data `{:?}` but it could not be found", td);
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
        fs::create_dir_all(&dst)?;
        let mut walker = WalkBuilder::new(src);
        walker.git_ignore(!copy_ignored_test_data);
        walker.add_ignore(ignore_file);
        for entry in walker.build() {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let path = entry.path().strip_prefix(src)?;
            if path.components().any(|comp| comp.as_ref() == "target") {
                continue;
            }
            let target = dst.join(path);
            if metadata.is_dir() {
                if target.exists() && !target.is_dir() {
                    fs::remove_dir_all(&target)?;
                }
                &fs::create_dir_all(&target)?;
            } else {
                if target.exists() && !target.is_file() {
                    fs::remove_dir_all(&target)?;
                }
                if !target.exists() || target.metadata()?.len() != entry.metadata()?.len()
                    || target.metadata()?.modified()? < entry.metadata()?.modified()?
                    {
                        fs::copy(entry.path(), &target)?;
                    }
            }
        }
        Ok(())
    }
}