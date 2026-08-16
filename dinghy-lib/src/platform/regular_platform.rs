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

use anyhow::{anyhow, bail};
use fs_err::read_dir;
use log::trace;

pub struct RegularPlatform {
    pub configuration: PlatformConfiguration,
    pub id: String,
    pub rustc_triple: String,
    /// Cross toolchain for building and stripping; `None` on a runner-only
    /// platform, which ships prebuilt binaries and needs no toolchain.
    pub toolchain: Option<ToolchainConfig>,
}

impl Debug for RegularPlatform {
    fn fmt(&self, fmt: &mut Formatter) -> ::std::fmt::Result {
        write!(fmt, "{}", self.id)
    }
}

impl RegularPlatform {
    /// Assemble a platform. With no usable `toolchain_path` (absent, or no
    /// `bin/*-gcc`) the platform is runner-only: it cannot build or strip.
    pub fn new<P: AsRef<Path>>(
        configuration: PlatformConfiguration,
        id: String,
        rustc_triple: String,
        toolchain_path: Option<P>,
    ) -> Result<Box<dyn Platform>> {
        let toolchain = assemble_toolchain(&configuration, &rustc_triple, toolchain_path)?;
        Ok(Box::new(RegularPlatform {
            configuration,
            id,
            rustc_triple,
            toolchain,
        }))
    }

    pub fn new_with_tc(
        configuration: PlatformConfiguration,
        id: String,
        toolchain: ToolchainConfig,
    ) -> Result<Box<dyn Platform>> {
        Ok(Box::new(RegularPlatform {
            rustc_triple: toolchain.rustc_triple.clone(),
            configuration,
            id,
            toolchain: Some(toolchain),
        }))
    }
}

/// The platform's toolchain, or `None` when none is usable (runner-only).
fn assemble_toolchain<P: AsRef<Path>>(
    configuration: &PlatformConfiguration,
    rustc_triple: &str,
    toolchain_path: Option<P>,
) -> Result<Option<ToolchainConfig>> {
    if let Some(prefix) = configuration.deb_multiarch.clone() {
        return Ok(Some(ToolchainConfig {
            bin_dir: "/usr/bin".into(),
            rustc_triple: rustc_triple.to_owned(),
            root: "/".into(),
            sysroot: Some("/".into()),
            cc: "gcc".to_string(),
            cxx: "c++".to_string(),
            binutils_prefix: prefix.clone(),
            cc_prefix: prefix,
        }));
    }
    let Some(toolchain_path) = toolchain_path else {
        return Ok(None);
    };
    let toolchain_path = toolchain_path.as_ref();
    let toolchain_bin_path = toolchain_path.join("bin");

    let Ok(entries) = read_dir(&toolchain_bin_path) else {
        trace!("no toolchain at {toolchain_bin_path:?}; platform is remote-run only");
        return Ok(None);
    };
    let mut bin: Option<PathBuf> = None;
    let mut prefix: Option<String> = None;
    for file in entries {
        let file = file?;
        if file.file_name().to_string_lossy().ends_with("-gcc")
            || file.file_name().to_string_lossy().ends_with("-gcc.exe")
        {
            bin = Some(toolchain_bin_path.clone());
            prefix = Some(
                file.file_name()
                    .to_string_lossy()
                    .replace(".exe", "")
                    .replace("-gcc", ""),
            );
            break;
        }
    }
    let (Some(bin_dir), Some(tc_triple)) = (bin, prefix) else {
        trace!("no bin/*-gcc in {toolchain_bin_path:?}; platform is remote-run only");
        return Ok(None);
    };
    let sysroot = find_sysroot(toolchain_path)?;

    Ok(Some(ToolchainConfig {
        bin_dir,
        rustc_triple: rustc_triple.to_owned(),
        root: toolchain_path.into(),
        sysroot,
        cc: "gcc".to_string(),
        cxx: "c++".to_string(),
        binutils_prefix: tc_triple.clone(),
        cc_prefix: tc_triple,
    }))
}

impl Display for RegularPlatform {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        match &self.toolchain {
            Some(toolchain) => write!(f, "{:?}", toolchain.root),
            None => write!(f, "{} (remote-run only)", self.rustc_triple),
        }
    }
}

impl Platform for RegularPlatform {
    fn setup_env(&self, project: &Project, setup_args: &SetupArgs) -> Result<()> {
        // Cleanup environment
        set_all_env(&[("LIBRARY_PATH", ""), ("LD_LIBRARY_PATH", "")]);
        // Set custom env variables specific to the platform
        set_all_env(&self.configuration.env());

        let Some(toolchain) = &self.toolchain else {
            return Ok(());
        };

        if let Some(sr) = &toolchain.sysroot {
            Overlayer::overlay(&self.configuration, self, project, &sr)?;
        }

        toolchain.setup_cc(&self.id, &toolchain.cc_executable(&toolchain.cc))?;

        if Path::new(&toolchain.binutils_executable("ar")).exists() {
            toolchain.setup_tool("AR", &toolchain.binutils_executable("ar"))?;
        }
        if Path::new(&toolchain.binutils_executable("as")).exists() {
            toolchain.setup_tool("AS", &toolchain.binutils_executable("as"))?;
        }
        if Path::new(&toolchain.cc_executable(&toolchain.cxx)).exists() {
            toolchain.setup_tool("CXX", &toolchain.cc_executable(&toolchain.cxx))?;
        }
        if Path::new(&toolchain.cc_executable("cpp")).exists() {
            toolchain.setup_tool("CPP", &toolchain.cc_executable("cpp"))?;
        }
        if Path::new(&toolchain.binutils_executable("gfortran")).exists() {
            toolchain.setup_tool("FC", &toolchain.binutils_executable("gfortran"))?;
        }
        trace!("Setup linker...");
        toolchain.setup_linker(
            &self.id,
            &toolchain.generate_linker_command(&setup_args),
            &project.metadata.workspace_root,
        )?;

        trace!("Setup pkg-config");
        toolchain.setup_pkg_config()?;
        trace!("Setup sysroot...");
        toolchain.setup_sysroot();
        trace!("Setup shims...");
        toolchain.shim_executables(&self.id, &project.metadata.workspace_root)?;
        trace!("Setup runner...");
        toolchain.setup_runner(&self.id, setup_args)?;
        trace!("Setup target...");
        toolchain.setup_target()?;
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
        &self.rustc_triple
    }

    fn strip(&self, build: &mut Build) -> Result<()> {
        let Some(toolchain) = &self.toolchain else {
            bail!(
                "platform {} has no cross toolchain, so it cannot strip; it supports \
                 remote execution only. Drop --strip or configure a toolchain.",
                self.id
            );
        };
        build.runnable = platform::strip_runnable(
            &build.runnable,
            Command::new(toolchain.binutils_executable("strip")),
        )?;

        Ok(())
    }

    fn sysroot(&self) -> Result<Option<std::path::PathBuf>> {
        Ok(self.toolchain.as_ref().and_then(|it| it.sysroot.clone()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PlatformConfiguration;

    #[test]
    fn toolchain_less_platform_assembles_for_remote_run() {
        let mut conf = PlatformConfiguration::empty();
        conf.rustc_triple = Some("riscv64gc-unknown-linux-musl".into());
        let pf = RegularPlatform::new(
            conf,
            "riscv".to_string(),
            "riscv64gc-unknown-linux-musl".to_string(),
            None::<PathBuf>,
        )
        .unwrap();
        assert_eq!(pf.rustc_triple(), "riscv64gc-unknown-linux-musl");
        assert_eq!(pf.sysroot().unwrap(), None);
        assert!(!pf.is_host());
    }

    #[test]
    fn deb_multiarch_yields_a_system_toolchain() {
        let mut conf = PlatformConfiguration::empty();
        conf.deb_multiarch = Some("riscv64-linux-gnu".into());
        let pf = RegularPlatform::new(
            conf,
            "riscv".to_string(),
            "riscv64gc-unknown-linux-gnu".to_string(),
            None::<PathBuf>,
        )
        .unwrap();
        assert_eq!(pf.sysroot().unwrap(), Some(PathBuf::from("/")));
    }
}
