use dinghy_build::build_env::set_all_env;
use overlay::Overlayer;
use platform;
use project::Project;
use std::collections::HashMap;
use std::fmt::{ Debug, Display, Formatter };
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use toolchain::ToolchainConfig;
use Build;
use BuildArgs;
use config::PlatformConfiguration;
use Device;
use Platform;
use Result;

pub struct RegularPlatform {
//    compiler: Arc<Compiler>,
    pub configuration: PlatformConfiguration,
    pub id: String,
    pub toolchain: ToolchainConfig,
}

impl Debug for RegularPlatform {
    fn fmt(&self, fmt: &mut Formatter) -> ::std::fmt::Result {
        write!(fmt, "{}", self.id)
    }
}

impl RegularPlatform {
    pub fn new<P: AsRef<Path>>(/*compiler: &Arc<Compiler>, */
                               configuration: PlatformConfiguration,
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
//            compiler: compiler.clone(),
            configuration,
            id,
            toolchain: ToolchainConfig {
                bin,
                rustc_triple,
                root: toolchain_path.into(),
                sysroot,
                toolchain_triple: tc_triple,
            },
        }))
    }
}

impl Display for RegularPlatform {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(f, "{:?}", self.toolchain.root)
    }
}

impl Platform for RegularPlatform {
    fn build(&self, project: &Project, build_args: &BuildArgs) -> Result<Build> {
        let mut env = HashMap::new();
        // Cleanup environment
        env.insert("LIBRARY_PATH".into(), None);
        env.insert("LD_LIBRARY_PATH".into(), None);

        // Set custom env variables specific to the platform
        for (k, v) in self.configuration.env() {
            env.insert(k, Some(v));
        }

        Overlayer::overlay(&self.configuration, self, project, &self.toolchain.sysroot)?;

        self.toolchain.setup_cc(&self.id, &self.toolchain.executable("gcc"), &mut env)?;

        if Path::new(&self.toolchain.executable("ar")).exists() {
            self.toolchain.setup_tool("AR", &self.toolchain.executable("ar"), &mut env)?;
        }
        if Path::new(&self.toolchain.executable("as")).exists() {
            self.toolchain.setup_tool("AS", &self.toolchain.executable("as"), &mut env)?;
        }
        if Path::new(&self.toolchain.executable("c++")).exists() {
            self.toolchain.setup_tool("CXX", &self.toolchain.executable("c++"), &mut env)?;
        }
        if Path::new(&self.toolchain.executable("cpp")).exists() {
            self.toolchain.setup_tool("CPP", &self.toolchain.executable("cpp"), &mut env)?;
        }
        if Path::new(&self.toolchain.executable("gfortran")).exists() {
            self.toolchain.setup_tool("FC", &self.toolchain.executable("gfortran"), &mut env)?;
        }

        let mut linker_cmd = self.toolchain.executable("gcc");
        linker_cmd.push_str(" ");
        if build_args.verbose { linker_cmd.push_str("-Wl,--verbose -v") }
        linker_cmd.push_str(&format!(" --sysroot {}", self.toolchain.sysroot.display()));
        for forced_overlay in &build_args.forced_overlays {
            linker_cmd.push_str(" -l");
            linker_cmd.push_str(&forced_overlay);
            // TODO Add -L
        }
        self.toolchain.setup_linker(&self.id, &linker_cmd, &mut env)?;

        self.toolchain.setup_pkg_config(&mut env)?;
        self.toolchain.setup_sysroot();
        self.toolchain.shim_executables(&self.id)?;

        ::cargo::call(build_args, self.rustc_triple(), env)
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

    fn strip(&self, build: &Build) -> Result<()> {
        for runnable in &build.runnables {
            platform::strip_runnable(runnable, Command::new(self.toolchain.executable("strip")))?;
        }
        Ok(())
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
