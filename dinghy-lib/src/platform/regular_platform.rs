use compiler::Compiler;
use compiler::CompileMode;
use config::PlatformConfiguration;
use dinghy_helper::build_env::set_all_env;
use overlay::Overlayer;
use overlay::overlay_work_dir;
use std::fmt::Display;
use std::path::Path;
use std::path::PathBuf;
use toolchain::ToolchainConfig;
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
        let sysroot = RegularPlatform::find_sysroot(&toolchain_path)?;

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

    fn find_sysroot<P: AsRef<Path>>(toolchain_path: P) -> Result<PathBuf> {
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
}

impl Display for RegularPlatform {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(f, "{:?}", self.toolchain.root)
    }
}

impl Platform for RegularPlatform {
    fn build(&self, compiler: &Compiler, compile_mode: CompileMode) -> Result<Vec<Runnable>> {
        // Cleanup environment
        set_all_env(&[
            ("LIBRARY_PATH", ""),
            ("LD_LIBRARY_PATH", ""),
        ]);
        // Set custom env variables specific to the platform
        set_all_env(&self.configuration.env());

        Overlayer::new(self, &self.toolchain.sysroot, overlay_work_dir(compiler, self)?)
            .overlay(&self.configuration, compiler.project_dir()?)?;

        self.toolchain.setup_ar(&self.toolchain.executable("ar"))?;
        self.toolchain.setup_cc(&self.id, &self.toolchain.executable("gcc"))?;
        self.toolchain.setup_linker(&self.id,
                                    &format!("{} --sysroot {}", // TODO Debug  -Wl,--verbose -v
                                             &self.toolchain.executable("gcc"),
                                             &self.toolchain.sysroot.display()))?;
        self.toolchain.setup_pkg_config()?;
        self.toolchain.setup_sysroot();
        self.toolchain.shim_executables(&self.id)?;

        compiler.build(self, compile_mode)
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn is_compatible_with(&self, device: &Device) -> bool {
        device.is_compatible_with_regular_platform(self)
    }

    fn rustc_triple(&self) -> Option<&str> {
        Some(&self.toolchain.rustc_triple)
    }
}
