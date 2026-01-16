use crate::platform::regular_platform::RegularPlatform;
use crate::toolchain::{create_shim, ToolchainConfig};
use crate::Result;
use crate::{Build, Device, Platform, PlatformConfiguration, Project, SetupArgs};
use anyhow::{anyhow, Context};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub enum OhosArch {
    Aarch64,
    Armv7,
    X86_64,
}

#[derive(Debug)]
pub struct OhosPlatform {
    regular_platform: Box<dyn Platform>,
    arch: OhosArch,
    toolchain_config: ToolchainConfig,
    /// Will use it some day, it's inevitable.
    #[allow(dead_code)]
    ndk_major_version: usize,
    ndk_path: PathBuf,
}

impl OhosPlatform {
    pub fn new(
        configuration: PlatformConfiguration,
        arch: OhosArch,
        id: String,
        toolchain_config: ToolchainConfig,
        ndk_major_version: usize,
        ndk_path: PathBuf,
    ) -> Result<Box<dyn Platform>> {
        Ok(Box::new(Self {
            regular_platform: RegularPlatform::new_with_tc(
                configuration,
                id,
                toolchain_config.clone(),
            )?,
            arch,
            toolchain_config,
            ndk_major_version,
            ndk_path,
        }))
    }
}

// see https://doc.rust-lang.org/rustc/platform-support/openharmony.html
fn ohos_ndk_tool_wrapper(arch: OhosArch, tool: &str, ndk_path: &Path) -> String {
    let tools_path = format!("{}/llvm/bin/{}", ndk_path.display(), tool);
    let sysroot = format!("{}/sysroot", ndk_path.display());
    let content = match arch {
        OhosArch::Aarch64 => format!(
            r###"
exec "{tools_path}" \
    -target aarch64-linux-ohos \
    --sysroot="{sysroot}" \
    -D__MUSL__ \
    "$@"
"###,
        ),

        OhosArch::Armv7 => format!(
            r###"
exec "{tools_path}" \
    -target arm-linux-ohos \
    --sysroot="{sysroot}" \
    -D__MUSL__ \
    -march=armv7-a \
    -mfloat-abi=softfp \
    -mtune=generic-armv7-a \
    -mthumb \
    "$@"
"###,
        ),
        OhosArch::X86_64 => format!(
            r###"
exec "{tools_path}" \
    -target x86_64-linux-ohos \
    --sysroot="{sysroot}" \
    -D__MUSL__ \
    "$@"
"###,
        ),
    };
    content
}

impl Platform for OhosPlatform {
    fn setup_env(&self, project: &Project, setup_args: &SetupArgs) -> anyhow::Result<()> {
        self.regular_platform.setup_env(project, setup_args)?;

        let shim_creator = |tool: &str| {
            let content = ohos_ndk_tool_wrapper(self.arch, tool, &self.ndk_path);
            create_shim(
                &project.metadata.workspace_root,
                self.regular_platform.rustc_triple(),
                &self.regular_platform.id(),
                tool,
                &content,
            )
            .with_context(|| anyhow!("Create {} shim failed.", tool))
        };

        let cc = shim_creator("clang").context("Create clang shim failed.")?;
        let cxx = shim_creator("clang++").context("Create clang++ shim failed.")?;
        self.toolchain_config
            .setup_tool("CC", &cc.to_string_lossy())?;
        self.toolchain_config
            .setup_linker_raw(&cc.to_string_lossy());
        self.toolchain_config
            .setup_tool("CXX", &cxx.to_string_lossy())?;
        self.toolchain_config
            .setup_tool("CPP", &cxx.to_string_lossy())?;
        self.toolchain_config
            .setup_tool("AR", &self.toolchain_config.naked_executable("llvm-ar"))?;
        self.toolchain_config
            .setup_tool("AS", &self.toolchain_config.naked_executable("llvm-as"))?;
        self.toolchain_config.setup_tool(
            "RANLIB",
            &self.toolchain_config.naked_executable("llvm-ranlib"),
        )?;

        Ok(())
    }

    fn id(&self) -> String {
        self.regular_platform.id()
    }

    fn is_compatible_with(&self, device: &dyn Device) -> bool {
        self.regular_platform.is_compatible_with(device)
    }

    fn is_host(&self) -> bool {
        self.regular_platform.is_host()
    }

    fn rustc_triple(&self) -> &str {
        self.regular_platform.rustc_triple()
    }

    fn strip(&self, build: &mut Build) -> anyhow::Result<()> {
        self.regular_platform.strip(build)
    }

    fn sysroot(&self) -> anyhow::Result<Option<PathBuf>> {
        self.regular_platform.sysroot()
    }
}
