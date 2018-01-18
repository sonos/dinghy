use dinghy_helper::build_env::append_path_to_target_env;
use dinghy_helper::build_env::envify;
use errors::*;
use itertools::Itertools;
use std::io::Write;
use std::path::PathBuf;
use walkdir::WalkDir;

use cargo_facade::CargoFacade;
//use config::OverlayConfiguration;
use config::PlatformConfiguration;
use dinghy_helper::build_env::set_env_ifndef;
//use std::env::home_dir;
use std::fs::create_dir_all;
use std::fs::remove_dir_all;
use std::fs::File;
use std::path::Path;
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
    pub temp_dir: PathBuf,
    pub rustc_triple: String,
    pub sysroot: String,
//    pub overlays: Vec<Overlay>,
}

//        , platform: &Platform, cargo_facade: &CargoFacade
//        let pkg_config_temp_dir = get_or_find_temp_path(cargo_facade,
//                                                        platform,
//                                                        &self.rustc_triple)?.join("pkgconfig");
impl Overlayer {
    pub fn apply_overlay(&self, id: &str, overlay_list: &[&Overlay]) -> Result<()> {
        remove_dir_all(&self.temp_dir)?;
        create_dir_all(&self.temp_dir)?;
        append_path_to_target_env("PKG_CONFIG_LIBDIR", &self.rustc_triple, &self.temp_dir);

        for overlay in overlay_list {
            debug!("Overlaying '{}'", overlay.id.as_str());
            let mut has_pkg_config_files = false;

            for pkg_config_path in WalkDir::new(&overlay.path)
                .into_iter()
                .filter_map(|entry| entry.ok()) // Ignore unreadable directories, maybe could warn...
                .filter(|entry| entry.file_type().is_dir())
                .filter(|dir| dir.file_name() == "pkgconfig" || contains_file_with_ext(dir.path(), ".pc"))
                .filter_map(|e| e.path().to_str().map(|it| it.to_string())) {
                debug!("Discovered pkg-config directory '{}'", pkg_config_path.as_str());
                append_path_to_target_env("PKG_CONFIG_LIBDIR", &self.rustc_triple, pkg_config_path);
                has_pkg_config_files = true;
            }

            // Generate a default pkg-config file if none found.
            if !has_pkg_config_files {
                debug!("No pkg-config pc file found for {}", overlay.id);
                let generated_pkg_config_file = self.temp_dir.join(format!("{}.pc", id));
                generate_pkg_config_file(generated_pkg_config_file.as_path(),
                                         overlay.id.as_str(),
                                         WalkDir::new(&overlay.path).max_depth(1)
                                             .into_iter()
                                             .filter_map(|entry| entry.ok()) // Ignore unreadable files, maybe could warn...
                                             .filter(|entry| file_has_ext(entry.path(), ".so"))
                                             .filter_map(|e| lib_name(e.path()).ok())
                                             .collect_vec()
                                             .as_slice())
                    .chain_err(|| format!("Dinghy couldn't generate pkg-config pc file {}",
                                          generated_pkg_config_file.as_path().display()))?;
            }

            // Override the 'prefix' pkg-config variable for the specified overlay only.
            set_env_ifndef(envify(format!("PKG_CONFIG_{}_PREFIX", overlay.id)),
                           path_between(&self.sysroot, &overlay.path));
        }
        Ok(())
    }

    fn find_overlays<P: AsRef<Path>>(target_path: P, id: &str, configuration: &PlatformConfiguration) -> Result<Vec<Overlay>> {
        let mut path_to_try = vec![];
        let target_path = target_path.as_ref().to_path_buf();
        let mut current_path = target_path.as_path();
        while current_path.parent().is_some() {
            path_to_try.push(current_path.join(".dinghy").join("overlay").join(id));
            current_path = current_path.parent().unwrap();
        }

        Ok(Overlayer::from_conf(configuration)?
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
}

fn contains_file_with_ext(dir_path: &Path, ext: &str) -> bool {
    if !dir_path.is_dir() { return false; };
    if let Ok(path) = dir_path.read_dir() {
        for file in path {
            if let Ok(file) = file {
                if file.file_name().to_string_lossy().ends_with(ext) {
                    return true;
                }
            }
        }
    }
    false
}

fn destructure_path<P: AsRef<Path>>(path: P) -> Option<(PathBuf, String)> {
    let path = path.as_ref();
    path.file_name()
        .and_then(|it| it.to_str())
        .map(|name| (path.to_path_buf(), name.to_string()))
}

fn file_has_ext(file_path: &Path, ext: &str) -> bool {
    file_path.is_file() && file_path.file_name()
        .and_then(|it| it.to_str())
        .map(|it| it.ends_with(ext))
        .unwrap_or(false)
}

fn generate_pkg_config_file<P: AsRef<Path>, T: AsRef<str>>(pc_file_path: P, name: &str, libs: &[T]) -> Result<()> {
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

fn get_or_find_temp_path(cargo_facade: &CargoFacade, platform: &Platform, rustc_triple: &str) -> Result<PathBuf> {
    Ok(cargo_facade.target_dir(rustc_triple)?.join(platform.id()))
}

fn lib_name(file_path: &Path) -> Result<String> {
    let file_name = file_path.file_name()
        .and_then(|it| it.to_str())
        .ok_or(format!("'{}' doesn't point to a valid lib name", file_path.display()))?;

    let start_index = if file_name.starts_with("lib") { 3 } else { 0 };
    let end_index = file_name.find(".so").unwrap_or(file_name.len());
    if start_index == end_index {
        bail!("'{}' doesn't point to a valid lib name", file_path.display());
    } else {
        Ok(file_name[start_index..end_index].to_string())
    }
}

fn path_between<P1: AsRef<Path>, P2: AsRef<Path>>(from: P1, to: P2) -> PathBuf {
    let mut path = PathBuf::new();
    for _ in from.as_ref() { path.push("/.."); }
    for dir in to.as_ref() { path.push(dir); }
    path
}
