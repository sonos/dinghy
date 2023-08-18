use crate::config::PlatformConfiguration;
use crate::overlay::Overlayer;
use crate::platform;
use crate::project::Project;
use crate::toolchain::ToolchainConfig;
use crate::Build;
use crate::Device;
use crate::Platform;
use crate::Result;
use crate::SetupArgs;
use dinghy_build::build_env::set_all_env;
use std::fmt::{Debug, Display, Formatter};
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, Context};
use log::trace;

pub struct RegularPlatform {
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
    pub fn new<P: AsRef<Path>>(
        configuration: PlatformConfiguration,
        id: String,
        rustc_triple: String,
        toolchain_path: P,
    ) -> Result<Box<dyn Platform>> {
        if let Some(prefix) = configuration.deb_multiarch.clone() {
            return Ok(Box::new(RegularPlatform {
                configuration,
                id,
                toolchain: ToolchainConfig {
                    bin_dir: "/usr/bin".into(),
                    rustc_triple,
                    root: "/".into(),
                    sysroot: Some("/".into()),
                    cc: "gcc".to_string(),
                    cxx: "c++".to_string(),
                    binutils_prefix: prefix.clone(),
                    cc_prefix: prefix.clone(),
                },
            }));
        }
        let toolchain_path = toolchain_path.as_ref();
        let toolchain_bin_path = toolchain_path.join("bin");

        let mut bin: Option<PathBuf> = None;
        let mut prefix: Option<String> = None;
        for file in toolchain_bin_path.read_dir().with_context(|| {
            format!(
                "Couldn't find toolchain directory {}",
                toolchain_path.display()
            )
        })? {
            let file = file?;
            if file.file_name().to_string_lossy().ends_with("-gcc")
                || file.file_name().to_string_lossy().ends_with("-gcc.exe")
            {
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
        let bin_dir = bin.ok_or_else(|| anyhow!("no bin/*-gcc found in toolchain"))?;
        let tc_triple = prefix
            .ok_or_else(|| anyhow!("no gcc in toolchain"))?
            .to_string();
        let sysroot = find_sysroot(&toolchain_path)?;

        let toolchain = ToolchainConfig {
            bin_dir,
            rustc_triple,
            root: toolchain_path.into(),
            sysroot,
            cc: "gcc".to_string(),
            cxx: "c++".to_string(),
            binutils_prefix: tc_triple.clone(),
            cc_prefix: tc_triple,
        };
        Self::new_with_tc(configuration, id, toolchain)
    }

    pub fn new_with_tc(
        configuration: PlatformConfiguration,
        id: String,
        toolchain: ToolchainConfig,
    ) -> Result<Box<dyn Platform>> {
        Ok(Box::new(RegularPlatform {
            configuration,
            id,
            toolchain,
        }))
    }
}

impl Display for RegularPlatform {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(f, "{:?}", self.toolchain.root)
    }
}

impl Platform for RegularPlatform {
    fn setup_env(&self, project: &Project, setup_args: &SetupArgs) -> Result<()> {
        // Cleanup environment
        set_all_env(&[("LIBRARY_PATH", ""), ("LD_LIBRARY_PATH", "")]);
        // Set custom env variables specific to the platform
        set_all_env(&self.configuration.env());

        if let Some(sr) = &self.toolchain.sysroot {
            Overlayer::overlay(&self.configuration, self, project, &sr)?;
        }

        self.toolchain
            .setup_cc(&self.id, &self.toolchain.cc_executable(&self.toolchain.cc))?;

        if Path::new(&self.toolchain.binutils_executable("ar")).exists() {
            self.toolchain
                .setup_tool("AR", &self.toolchain.binutils_executable("ar"))?;
        }
        if Path::new(&self.toolchain.binutils_executable("as")).exists() {
            self.toolchain
                .setup_tool("AS", &self.toolchain.binutils_executable("as"))?;
        }
        if Path::new(&self.toolchain.cc_executable(&self.toolchain.cxx)).exists() {
            self.toolchain
                .setup_tool("CXX", &self.toolchain.cc_executable(&self.toolchain.cxx))?;
        }
        if Path::new(&self.toolchain.cc_executable("cpp")).exists() {
            self.toolchain
                .setup_tool("CPP", &self.toolchain.cc_executable("cpp"))?;
        }
        if Path::new(&self.toolchain.binutils_executable("gfortran")).exists() {
            self.toolchain
                .setup_tool("FC", &self.toolchain.binutils_executable("gfortran"))?;
        }
        trace!("Setup linker...");
        self.toolchain.setup_linker(
            &self.id,
            &self.toolchain.generate_linker_command(&setup_args),
            &project.metadata.workspace_root,
        )?;

        trace!("Setup pkg-config");
        self.toolchain.setup_pkg_config()?;
        trace!("Setup sysroot...");
        self.toolchain.setup_sysroot();
        trace!("Setup shims...");
        self.toolchain
            .shim_executables(&self.id, &project.metadata.workspace_root)?;
        trace!("Setup runner...");
        self.toolchain.setup_runner(&self.id, setup_args)?;
        trace!("Setup target...");
        self.toolchain.setup_target()?;
        Ok(())
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn is_compatible_with(&self, device: &dyn Device) -> bool {
        device.is_compatible_with_regular_platform(self)
    }

    fn is_host(&self) -> bool {
        false
    }

    fn rustc_triple(&self) -> &str {
        &self.toolchain.rustc_triple
    }

    fn strip(&self, build: &mut Build) -> Result<()> {
        build.runnable = platform::strip_runnable(
            &build.runnable,
            Command::new(self.toolchain.binutils_executable("strip")),
        )?;

        Ok(())
    }

    fn sysroot(&self) -> Result<Option<std::path::PathBuf>> {
        Ok(self.toolchain.sysroot.clone())
    }
}

fn find_sysroot<P: AsRef<Path>>(toolchain_path: P) -> Result<Option<PathBuf>> {
    let toolchain = toolchain_path.as_ref();
    let immediate = toolchain.join("sysroot");
    if immediate.is_dir() {
        let sysroot = immediate
            .to_str()
            .ok_or_else(|| anyhow!("sysroot is not utf-8"))?;
        return Ok(Some(sysroot.into()));
    }
    for subdir in toolchain.read_dir()? {
        let subdir = subdir?;
        let maybe = subdir.path().join("sysroot");
        if maybe.is_dir() {
            let sysroot = maybe
                .to_str()
                .ok_or_else(|| anyhow!("sysroot is not utf-8"))?;
            return Ok(Some(sysroot.into()));
        }
    }
    Ok(None)
}
