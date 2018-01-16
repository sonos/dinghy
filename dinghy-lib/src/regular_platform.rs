use cargo_facade::CargoFacade;
use cargo_facade::CompileMode;
use clap::ArgMatches;
use config::OverlayConfiguration;
use config::PlatformConfiguration;
use dinghy_helper::build_env::append_path_to_target_env;
use dinghy_helper::build_env::envify_key;
use dinghy_helper::build_env::set_all_env;
use dinghy_helper::build_env::set_env_ifndef;
use std::env::home_dir;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;
use toolchain::ToolchainConfig;
use walkdir::WalkDir;
use Device;
use Platform;
use Result;
use Runnable;

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
    fn build(&self, compile_mode: CompileMode, matches: &ArgMatches) -> Result<Vec<Runnable>> {
        set_all_env(self.configuration.env().as_slice());

        let mut whatever_overlays = vec![];
        let overlay_dir = home_dir().map(|it| it.join(".dinghy").join("overlay").join(self.id.as_str()));
//        error!("JJJJJ");
        if let Some(overlay_root_dir) = overlay_dir {
            if overlay_root_dir.is_dir() {
                if let Ok(overlay_dir_list) = overlay_root_dir.read_dir() {
                    for overlay_dir in overlay_dir_list {
                        if let Ok(overlay_dir) = overlay_dir {
//                        let id = overlay_dir.file_name().to_string();
//                        auto_detected_overlays.push((overlay_dir.file_name().into_string(),
//                                                     overlay_dir.path()));
                            let toto = (overlay_dir.file_name().to_string_lossy().to_string(),//.into_string(),
                                        OverlayConfiguration {
                                            path: overlay_dir.path().to_string_lossy().to_string(),
                                            scope: Some("app".to_string()),
                                        });
//                            error!("KKKKKK {:?}", &toto);
                            whatever_overlays.push(toto);
                        }
                    }
                } else {
                    debug!("Couldn't read overlay directory {}", overlay_root_dir.display());
//                    ::std::iter::empty()
                }
            }
        };
//        error!("LLLLL");

//        Box::new(whatever_overlays.iter()).extend(
//            Box::new(self.configuration.overlays.as_ref()
//                .unwrap_or(::std::collections::HashMap::new())
//                .map(|(id, conf)| (id.to_string(), (*conf).clone()))
//                .iter())
//
//        );
//        whatever_overlays.append(
//
//        );
        whatever_overlays.extend(self.configuration.overlays.as_ref()
                                     .unwrap_or(&::std::collections::HashMap::new())
                                     .into_iter()
                                     .map(|(id, conf)| (id.to_string(), (*conf).clone()))
//            .map(|it| Box::new(it))
        )
//            .unwrap_or(Box::new(::std::iter::empty()))
//            .unwrap_or(::std::collections::HashMap::new())
        ;
//        error!("MMMM {:?}", whatever_overlays);
//        Box::new(whatever_overlays.iter()).extend(
//            Box::new(self.configuration.overlays.as_ref()
//                .unwrap_or(::std::collections::HashMap::new())
//                .map(|(id, conf)| (id.to_string(), (*conf).clone()))
//                .iter())

//        );

//        whatever_overlays.extend(
//            self.configuration.overlays.as_ref()
//                .unwrap_or(::std::collections::HashMap::new())
//                .map(|(id, conf)| (id.to_string(), (*conf).clone())));
//        if let Some(home_dir) = home_dir() {
//            home_dir.join(".dinghy").join("overlay").join(self.id.as_str())
//                .read_dir()
//                .map
//                .filter(|it| it.is_dir())
//            ;
//        }

//        error!("MMMMM1 {:?}", &overlays);
//        if let Some(overlays) = self.configuration.overlays.as_ref() {
//            for overlay in overlays
//            whatever_overlays.iter().extends(overlays);
//            auto_detected_overlays.iter().extends(overlays);
//            error!("MMMMM {:?}", &overlays);
        for (id, conf) in whatever_overlays {
//                error!("NNNN {:?} {:?}", &id, &conf);
            for pkg_config_path in WalkDir::new(&conf.path)
                .into_iter()
//                    .map(|it| {
//                        error!("PPPPPP {:?}", &it);
//                        it
//                    })
                .filter_map(|e| e.ok()) // Ignore unreadable files, maybe could warn...
                .filter(|e| e.file_name() == "pkgconfig" && e.file_type().is_dir())
                .map(|e| e.path().to_string_lossy().into_owned()) {
//                    error!("OOOO {:?}", &pkg_config_path);
                append_path_to_target_env("PKG_CONFIG_LIBDIR",
                                          self.toolchain.rustc_triple.as_ref(),
                                          pkg_config_path);
            }

            let mut overlay_pkg_config_path = String::from("/");
            PathBuf::from(self.toolchain.sysroot.as_str())
                .iter()
                .fold(&mut overlay_pkg_config_path, |acc, _| {
                    acc.push_str("../");
                    acc
                });
            overlay_pkg_config_path.push_str(&conf.path);

            set_env_ifndef(format!("PKG_CONFIG_{}_PREFIX", envify_key(id.as_str())),
                           overlay_pkg_config_path.as_str());
        }
//        }

        self.toolchain.setup_ar(self.toolchain.executable("ar").as_str())?;
        self.toolchain.setup_cc(self.id.as_str(), self.toolchain.executable("gcc").as_str())?;
        self.toolchain.setup_linker(self.id.as_str(),
                                    format!("{} --sysroot {}",
                                            self.toolchain.executable("gcc").as_str(),
                                            self.toolchain.sysroot.as_str()).as_str())?;
        self.toolchain.setup_pkg_config()?;
        self.toolchain.setup_sysroot();
        self.toolchain.shim_executables(self.id.as_str())?;

        CargoFacade::from_args(matches).build(compile_mode, Some(self.toolchain.rustc_triple.as_str()))
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn is_compatible_with(&self, device: &Device) -> bool {
        device.is_compatible_with_regular_platform(self)
    }
}
