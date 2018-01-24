use compiler::CompilationResult;
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
use utils::is_library;
use walkdir::WalkDir;
use Build;
use Device;
use Platform;
use Result;

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
        let sysroot = find_sysroot(&toolchain_path)?;

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

    fn find_dynamic_liraries(&self, compilation_result: &CompilationResult) -> Result<Vec<PathBuf>> {
        Ok(self.toolchain.library_dirs(&self.id)?
            .iter()
            .chain(compilation_result.native_dirs.iter())
            .inspect(|path| debug!("Checking library path {:?}", path.display()))
            .filter(|path| !self.is_system_path(path).unwrap_or(true))
            .inspect(|path| debug!("{} is not a system library path", path.display()))
            .flat_map(|path| WalkDir::new(path).into_iter())
            .filter_map(|walk_entry| walk_entry.map(|it| it.path().to_path_buf()).ok())
            .filter(|path| path.is_file() && is_library(path))
            .inspect(|path| debug!("Found library {:?}", path.display()))
            .collect())
    }

    fn is_system_path(&self, path: &Path) -> Result<bool> {
        let ignored_path = vec![
            Path::new("/lib"),
            Path::new("/usr/lib"),
            Path::new("/usr/lib32"),
            Path::new("/usr/lib64"),
        ];
        let is_system_path = ignored_path.iter().any(|it| path.starts_with(it))
            || path.canonicalize()?.starts_with(&self.toolchain.sysroot);
        Ok(is_system_path)
    }
}

impl Display for RegularPlatform {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(f, "{:?}", self.toolchain.root)
    }
}

impl Platform for RegularPlatform {
    fn build(&self, compiler: &Compiler, compile_mode: CompileMode) -> Result<Build> {
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
                                    &format!("{} -Wl,--verbose -v --sysroot {}", // TODO Debug  -Wl,--verbose -v
                                             &self.toolchain.executable("gcc"),
                                             &self.toolchain.sysroot.display()))?;
        self.toolchain.setup_pkg_config()?;
        self.toolchain.setup_sysroot();
        self.toolchain.shim_executables(&self.id)?;

        let compilation_result = compiler.build(self.rustc_triple(), compile_mode)?;
        Ok(Build {
            dynamic_libraries: self.find_dynamic_liraries(&compilation_result)?,
            runnables: compilation_result.runnables,
            target_path: compilation_result.target_path,
        })
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
