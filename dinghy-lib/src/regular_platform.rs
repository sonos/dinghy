use cargo_facade::CargoFacade;
use cargo_facade::CompileMode;
use clap::ArgMatches;
use std::fmt::Display;
use toolchain::ToolchainConfig;
use std::path;
use Device;
use Platform;
use Result;
use Runnable;

#[derive(Debug)]
pub struct RegularPlatform {
    pub id: String,
    pub toolchain: ToolchainConfig,
}

impl RegularPlatform {
    pub fn new<P: AsRef<path::Path>>(id: String, rustc_triple: String, toolchain_path: P) -> Result<Box<Platform>> {
        let toolchain_path = toolchain_path.as_ref();
        let toolchain_bin_path = toolchain_path.join("bin");

        let mut bin: Option<path::PathBuf> = None;
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

fn sysroot_in_toolchain<P: AsRef<path::Path>>(toolchain_path: P) -> Result<String> {
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
        self.toolchain.setup_ar(self.toolchain.executable("ar").as_str())?;
        self.toolchain.setup_cc(self.id.as_str(), self.toolchain.executable("gcc").as_str())?;
        self.toolchain.setup_linker(self.id.as_str(),
                                    format!("{} --sysroot {}",
                                            self.toolchain.executable("gcc").as_str(),
                                            self.toolchain.sysroot.as_str()).as_str())?;
        self.toolchain.setup_pkg_config()?;
        self.toolchain.setup_sysroot();
        self.toolchain.shim_executables(self.id.as_str())?;

        CargoFacade::from_args(matches)
            .build(compile_mode, Some(self.toolchain.rustc_triple.as_str()))
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn is_compatible_with(&self, device: &Device) -> bool {
        device.is_compatible_with_regular_platform(self)
    }
}
