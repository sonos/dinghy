use config::PlatformConfiguration;
use dinghy_helper::build_env::append_path_to_target_env;
use dinghy_helper::build_env::envify;
use dinghy_helper::build_env::set_env_ifndef;
use dinghy_helper::utils::path_between;
use errors::*;
use itertools::Itertools;
use project::Project;
use std::io::Write;
use std::path::PathBuf;
use std::env::home_dir;
use std::fs::create_dir_all;
use std::fs::remove_dir_all;
use std::fs::File;
use std::path::Path;
use utils::contains_file_with_ext;
use utils::file_has_ext;
use utils::destructure_path;
use utils::lib_name_from;
use walkdir::WalkDir;
use Platform;

#[derive(Clone, Debug)]
pub enum OverlayScope {
    Application,
    System,
}

#[derive(Clone, Debug)]
pub struct Overlay {
    pub id: String,
    pub path: PathBuf,
    pub scope: OverlayScope,
}

#[derive(Clone, Debug)]
pub struct Overlayer {
    platform_id: String,
    rustc_triple: Option<String>,
    sysroot: PathBuf,
    work_dir: PathBuf,
}

impl Overlayer {
    pub fn overlay<P: AsRef<Path>>(configuration: &PlatformConfiguration,
                                   platform: &Platform,
                                   project: &Project,
                                   sysroot: P) -> Result<()> {
        let overlayer = Overlayer {
            platform_id: platform.id().to_string(),
            rustc_triple: platform.rustc_triple().map(|it| it.to_string()),
            sysroot: sysroot.as_ref().to_path_buf(),
            work_dir: project.overlay_work_dir(platform)?,
        };

        let mut path_to_try = vec![];
        let project_path = project.project_dir()?;
        let mut current_path = project_path.as_path();
        while current_path.parent().is_some() {
            path_to_try.push(current_path.join(".dinghy").join("overlay").join(&overlayer.platform_id));
            if let Some(parent_path) = current_path.parent() {
                current_path = parent_path;
            } else {
                break;
            }
        }

        // Project may be outside home directory. So add it too.
        if let Some(dinghy_home_dir) = home_dir()
            .map(|it| it.join(".dinghy").join("overlay").join(&overlayer.platform_id)) {
            if !path_to_try.contains(&dinghy_home_dir) {
                path_to_try.push(dinghy_home_dir)
            }
        }

        overlayer.apply_overlay(Overlayer::from_conf(configuration)?
            .into_iter()
            .chain(path_to_try
                .into_iter()
                .flat_map(|path_to_try| Overlayer::from_directory(path_to_try).unwrap_or_default()))
            .unique_by(|overlay| overlay.id.clone())
            .collect_vec())
    }

    fn from_conf(configuration: &PlatformConfiguration) -> Result<Vec<Overlay>> {
        Ok(configuration.overlays.as_ref()
            .unwrap_or(&::std::collections::HashMap::new())
            .into_iter()
            .map(|(overlay_id, overlay_conf)| {
                Overlay {
                    id: overlay_id.to_string(),
                    path: PathBuf::from(overlay_conf.path.as_str()),
                    scope: OverlayScope::Application,
                }
            })
            .collect())
    }

    fn from_directory<P: AsRef<Path>>(overlay_root_dir: P) -> Result<Vec<Overlay>> {
        Ok(overlay_root_dir.as_ref()
            .read_dir()
            .chain_err(|| format!("Couldn't read overlay root directory '{}'.",
                                  overlay_root_dir.as_ref().display()))?
            .filter_map(|it| it.ok()) // Ignore invalid directories
            .map(|it| it.path())
            .filter(|it| it.is_dir())
            .filter_map(destructure_path)
            .map(|(overlay_dir_path, overlay_dir_name)| {
                Overlay {
                    id: overlay_dir_name,
                    path: overlay_dir_path.to_path_buf(),
                    scope: OverlayScope::Application,
                }
            })
            .collect())
    }

    fn apply_overlay<I>(&self, overlays: I) -> Result<()>
        where I: IntoIterator<Item=Overlay> {
        let pkg_config_env_var = self.rustc_triple.as_ref().map(|_| "PKG_CONFIG_LIBDIR")
            // Fallback on PKG_CONFIG_LIBPATH for host as it doesn't erase pkg_config paths
            .unwrap_or("PKG_CONFIG_LIBPATH");

        // Setup overlay work directory
        if let Err(error) = remove_dir_all(&self.work_dir) {
            if self.work_dir.exists() {
                warn!("Couldn't cleanup directory overlay work directory {} ({:?})", self.work_dir.display(), error)
            }
        }
        create_dir_all(&self.work_dir).chain_err(|| format!("Couldn't create overlay work directory {}.",
                                                            self.work_dir.display()))?;
        append_path_to_target_env(pkg_config_env_var, self.rustc_triple.as_ref(), &self.work_dir);

        for overlay in overlays {
            debug!("Overlaying '{}'", overlay.id.as_str());
            let mut has_pkg_config_files = false;

            let pkg_config_path_list = WalkDir::new(&overlay.path)
                .into_iter()
                .filter_map(|entry| entry.ok()) // Ignore unreadable directories, maybe could warn...
                .filter(|entry| entry.file_type().is_dir())
                .filter(|dir| dir.file_name() == "pkgconfig" || contains_file_with_ext(dir.path(), ".pc"))
                .map(|pkg_config_path| pkg_config_path.path().to_path_buf());

            for pkg_config_path in pkg_config_path_list {
                debug!("Discovered pkg-config directory '{}'", pkg_config_path.display());
                append_path_to_target_env(pkg_config_env_var, self.rustc_triple.as_ref(), pkg_config_path);
                has_pkg_config_files = true;
            }
            if !has_pkg_config_files {
                self.generate_pkg_config_file(&overlay)?;
                append_path_to_target_env(pkg_config_env_var, self.rustc_triple.as_ref(), &overlay.path);
            }

            // Override the 'prefix' pkg-config variable for the specified overlay only.
            set_env_ifndef(envify(format!("PKG_CONFIG_{}_PREFIX", overlay.id)),
                           path_between(&self.sysroot, &overlay.path));
        }
        Ok(())
    }

    fn generate_pkg_config_file(&self, overlay: &Overlay) -> Result<()> {
        fn write_pkg_config_file<P: AsRef<Path>, T: AsRef<str>>(pc_file_path: P, name: &str, libs: &[T]) -> Result<()> {
            debug!("Generating pkg-config pc file {}", pc_file_path.as_ref().display());
            let mut pc_file = File::create(pc_file_path)?;
            pc_file.write_all(b"prefix:/")?;
            pc_file.write_all(b"\nexec_prefix:${prefix}")?;
            pc_file.write_all(b"\nName: ")?;
            pc_file.write_all(name.as_bytes())?;
            pc_file.write_all(b"\nDescription: ")?;
            pc_file.write_all(name.as_bytes())?;
            pc_file.write_all(b"\nVersion: unspecified")?;
            pc_file.write_all(b"\nLibs: -L${prefix} ")?;
            for lib in libs {
                pc_file.write_all(b" -l")?;
                pc_file.write_all(lib.as_ref().as_bytes())?;
            }
            pc_file.write_all(b"\nCflags: -I${prefix}")?;
            Ok(())
        }

        let pc_file = self.work_dir.join(format!("{}.pc", self.platform_id));
        let lib_list = WalkDir::new(&overlay.path).max_depth(1)
            .into_iter()
            .filter_map(|entry| entry.ok()) // Ignore unreadable files, maybe could warn...
            .filter(|entry| file_has_ext(entry.path(), ".so"))
            .filter_map(|e| lib_name_from(e.path()).ok())
            .collect_vec();

        write_pkg_config_file(pc_file.as_path(), overlay.id.as_str(), &lib_list)
            .chain_err(|| format!("Dinghy couldn't generate pkg-config pc file {}",
                                  pc_file.as_path().display()))
    }
}
