use cargo_facade::CargoFacade;
use cargo_facade::CompileMode;
use config::OverlayConfiguration;
use config::PlatformConfiguration;
use dinghy_helper::build_env::append_path_to_target_env;
use dinghy_helper::build_env::envify_key;
use dinghy_helper::build_env::set_all_env;
use dinghy_helper::build_env::set_env;
use dinghy_helper::build_env::set_env_ifndef;
use itertools::Itertools;
use std::env::home_dir;
use std::fmt::Display;
use std::fs::create_dir_all;
use std::fs::remove_dir_all;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use toolchain::ToolchainConfig;
use walkdir::WalkDir;
use Device;
use Platform;
use Result;
use ResultExt;
use Runnable;

fn generate_pc_file<P: AsRef<Path>, T: AsRef<str>>(pc_file_path: P, name: &str, libs: &[T]) -> Result<()> {
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
//        let file_name = lib.file
        pc_file.write_all(b" -l")?;
        pc_file.write_all(lib.as_ref().as_bytes())?;
    }
    pc_file.write_all(b"\nCflags: -I${prefix}")?;
    Ok(())
}

#[derive(Debug)]
pub struct RegularPlatform {
    pub configuration: PlatformConfiguration,
    pub id: String,
    pub toolchain: ToolchainConfig,
}

impl RegularPlatform {
    pub fn new<P: AsRef<Path>>(configuration: PlatformConfiguration,
                               id: String,
                               rustc_triple: String,
                               toolchain_path: P) -> Result<Box<Platform>> {
        let toolchain_path = toolchain_path.as_ref();
        let toolchain_bin_path = toolchain_path.join("bin");

        let mut bin: Option<PathBuf> = None;
        let mut prefix: Option<String> = None;
        for file in toolchain_bin_path.read_dir().map_err(|_| format!("Couldn't find toolchain directory {}", toolchain_path.display()))? {
            let file = file?;
            if file.file_name().to_string_lossy().ends_with("-gcc")
                || file.file_name().to_string_lossy().ends_with("-gcc.exe") {
                bin = Some(toolchain_bin_path);
                prefix = Some(
                    file.file_name()
                        .to_string_lossy()
                        .replace(".exe", "")
                        .replace("-gcc", ""),
                );
                break;
            }
        }
        let bin = bin.ok_or("no bin/*-gcc found in toolchain")?;
        let tc_triple = prefix.ok_or("no gcc in toolchain")?.to_string();
        let sysroot = sysroot_in_toolchain(&toolchain_path)?;

        Ok(Box::new(RegularPlatform {
            configuration,
            id,
            toolchain: ToolchainConfig {
                bin,
                rustc_triple,
                root: toolchain_path.into(),
                sysroot,
                tc_triple,
            },
        }))
    }
}

impl Display for RegularPlatform {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(f, "{:?}", self.toolchain.root)
    }
}

fn sysroot_in_toolchain<P: AsRef<Path>>(toolchain_path: P) -> Result<String> {
    let toolchain = toolchain_path.as_ref();
    let immediate = toolchain.join("sysroot");
    if immediate.is_dir() {
        let sysroot = immediate.to_str().ok_or("sysroot is not utf-8")?;
        return Ok(sysroot.into());
    }
    for subdir in toolchain.read_dir()? {
        let subdir = subdir?;
        let maybe = subdir.path().join("sysroot");
        if maybe.is_dir() {
            let sysroot = maybe.to_str().ok_or("sysroot is not utf-8")?;
            return Ok(sysroot.into());
        }
    }
    Err(format!("no sysroot found in toolchain {:?}", toolchain))?
}

impl Platform for RegularPlatform {
    fn build(&self, cargo_facade: &CargoFacade, compile_mode: CompileMode) -> Result<Vec<Runnable>> {
        set_all_env(&[
            ("LIBRARY_PATH", ""),
            ("LD_LIBRARY_PATH", ""),
        ]);
        set_all_env(self.configuration.env().as_slice());

        let mut overlay_list = self.configuration.overlays.as_ref()
            .unwrap_or(&::std::collections::HashMap::new())
            .into_iter()
            .map(|(id, conf)| (id.to_string(), (*conf).clone()))
            .collect_vec();
        let overlay_dir = home_dir().map(|it| it.join(".dinghy").join("overlay").join(self.id.as_str()));
        if let Some(overlay_root_dir) = overlay_dir {
            if overlay_root_dir.is_dir() {
                if let Ok(overlay_dir_list) = overlay_root_dir.read_dir() {
                    for overlay_dir in overlay_dir_list {
                        if let Ok(overlay_dir) = overlay_dir {
                            let toto = (overlay_dir.file_name().to_string_lossy().to_string(),//.into_string(),
                                        OverlayConfiguration {
                                            path: overlay_dir.path().to_string_lossy().to_string(),
                                            scope: Some("app".to_string()),
                                        });
                            overlay_list.push(toto);
                        }
                    }
                } else {
                    debug!("Couldn't read overlay directory {}", overlay_root_dir.display());
                }
            }
        };

        fn contains_pc_file(dir_path: &Path) -> bool {
            if let Ok(path) = dir_path.read_dir() {
                for file in path {
                    if let Ok(file) = file {
                        if file.file_name().to_string_lossy().ends_with(".pc") {
                            return true;
                        }
                    }
                }
            }
            false
        };

        let pkg_config_temp_dir = get_or_find_temp_path(cargo_facade, self, self.toolchain.rustc_triple.as_str())?
            .join("pkgconfig");
        remove_dir_all(&pkg_config_temp_dir)?;
        create_dir_all(&pkg_config_temp_dir)?;
        append_path_to_target_env("PKG_CONFIG_LIBDIR",
                                  self.toolchain.rustc_triple.as_ref(),
                                  &pkg_config_temp_dir);

        for (id, conf) in overlay_list.into_iter().unique_by(|&(ref id, _)| id.clone()) {
            debug!("Overlaying '{}'", id.as_str());
            let mut pkg_config_found = false;
            for pkg_config_path in WalkDir::new(&conf.path)
                .into_iter()
                .filter_map(|e| e.ok()) // Ignore unreadable files, maybe could warn...
                .filter(|e| (e.file_name() == "pkgconfig" && e.file_type().is_dir()) || contains_pc_file(e.path()))
                .map(|e| e.path().to_string_lossy().into_owned()) {
                debug!("Discovered pkg-config directory '{}'", pkg_config_path.as_str());
                pkg_config_found = true;
                append_path_to_target_env("PKG_CONFIG_LIBDIR",
                                          self.toolchain.rustc_triple.as_ref(),
                                          pkg_config_path);
            }

            if !pkg_config_found {
                debug!("No pkg-config pc file found for {}", id);

                fn lib_name(file_name: &str) -> Option<String> {
                    let start_index = if file_name.starts_with("lib") { 3 } else { 0 };
                    let end_index = file_name.find(".so").unwrap_or(file_name.len());
                    if start_index == end_index {
                        None
                    } else {
                        Some(file_name[start_index..end_index].to_string())
                    }
                }

                // Generate pkg-config config file for current crate
                let pkg_config_file = pkg_config_temp_dir.join(format!("{}.pc", id));
                generate_pc_file(pkg_config_file.as_path(),
                                 id.as_str(),
                                 WalkDir::new(&conf.path).max_depth(1)
                                     .into_iter()
                                     .filter_map(|e| e.ok()) // Ignore unreadable files, maybe could warn...
                                     .filter(|e| e.file_name().to_str().map(|it| it.ends_with(".so")).unwrap_or(false))
                                     .filter(|e| e.path().is_file())
                                     .filter_map(|e| e.file_name().to_str().and_then(|it| lib_name(it)))
                                     .collect_vec()
                                     .as_slice())
                    .chain_err(|| format!("Dinghy couldn't generate pkg-config pc file {}", pkg_config_file.as_path().display()))?;
            }

            let mut overlay_pkg_config_path = String::from(""); // TODO Remove /
            PathBuf::from(self.toolchain.sysroot.as_str())
                .iter()
                .fold(&mut overlay_pkg_config_path, |acc, _| {
                    acc.push_str("/..");
                    acc
                });
            overlay_pkg_config_path.push_str(&conf.path);

            set_env_ifndef(format!("PKG_CONFIG_{}_PREFIX", envify_key(id.as_str())),
                           overlay_pkg_config_path.as_str());
        }

        self.toolchain.setup_ar(self.toolchain.executable("ar").as_str())?;
        self.toolchain.setup_cc(self.id.as_str(), self.toolchain.executable("gcc").as_str())?;
        self.toolchain.setup_linker(self.id.as_str(),
                                    format!("{} --sysroot {}", // TODO Debug  -Wl,--verbose -v
                                            self.toolchain.executable("gcc").as_str(),
                                            self.toolchain.sysroot.as_str()).as_str())?;
        self.toolchain.setup_pkg_config()?;
        self.toolchain.setup_sysroot();
        self.toolchain.shim_executables(self.id.as_str())?;

        append_path_to_target_env("PKG_CONFIG_LIBDIR",
                                  self.toolchain.rustc_triple.as_ref(),
                                  &pkg_config_temp_dir);

        cargo_facade.build(compile_mode, Some(self.toolchain.rustc_triple.as_str()))
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn is_compatible_with(&self, device: &Device) -> bool {
        device.is_compatible_with_regular_platform(self)
    }
}

pub fn get_or_find_temp_path(cargo_facade: &CargoFacade, platform: &Platform, rustc_triple: &str) -> Result<PathBuf> {
    Ok(cargo_facade.target_dir(rustc_triple)?.join(platform.id()))
//    let wd_path = ::cargo::util::important_paths::find_root_manifest_for_wd(None, &env::current_dir()?)?;
//    let root = wd_path.parent().ok_or("building at / ?")?;
}
//
//fn delete_dir_contents(directory: &Path) -> Result<()> {
//    if !directory.is_dir() { bail!("{:?} is not a directory", directory.display()) }
//
//    if let Ok(dir) = directory.read_dir() {
//        for entry in dir {
//            if let Ok(entry) = entry {
//                let path = entry.path();
//
//                if path.is_dir() {
//                    ::std::fs::remove_dir_all(path).expect("Failed to remove a dir");
//                } else {
//                    ::std::fs::remove_file(path).expect("Failed to remove a file");
//                }
//            };
//        }
//    };
//    Ok(())
//}