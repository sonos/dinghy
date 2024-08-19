use crate::platform::regular_platform::RegularPlatform;
use crate::toolchain::ToolchainConfig;
use crate::{platform, Result};
use crate::{Build, Device, Platform, PlatformConfiguration, Project, SetupArgs};
use dinghy_build::build_env::set_env;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug)]
pub struct AndroidPlatform {
    regular_platform: Box<dyn Platform>,
    toolchain_config: ToolchainConfig,
    ndk_major_version: usize,
    ndk_path: PathBuf,
    libclang_path: PathBuf,
}

impl AndroidPlatform {
    pub fn new(
        configuration: PlatformConfiguration,
        id: String,
        toolchain_config: ToolchainConfig,
        ndk_major_version: usize,
        ndk_path: PathBuf,
        libclang_path: PathBuf,
    ) -> Result<Box<dyn Platform>> {
        Ok(Box::new(Self {
            regular_platform: RegularPlatform::new_with_tc(
                configuration,
                id,
                toolchain_config.clone(),
            )?,
            toolchain_config,
            ndk_major_version,
            ndk_path,
            libclang_path,
        }))
    }
}

impl Platform for AndroidPlatform {
    fn setup_env(&self, project: &Project, setup_args: &SetupArgs) -> anyhow::Result<()> {
        self.regular_platform.setup_env(project, setup_args)?;

        if self.ndk_major_version >= 23 {
            log::trace!("Setup linker with android ndk23+ hack...");

            let hack_dir = project
                .target_dir(self.rustc_triple())
                .join(self.id())
                .join("ndk23-hack");

            std::fs::create_dir_all(&hack_dir)?;

            let mut hack_file = std::fs::File::create(hack_dir.join("libgcc.a"))?;

            hack_file.write_all("INPUT(-lunwind)".as_bytes())?;

            let mut linker_cmd = self.toolchain_config.generate_linker_command(&setup_args);

            linker_cmd.push_str(" -L");
            linker_cmd.push_str(hack_dir.canonicalize()?.to_str().unwrap());

            self.toolchain_config
                .setup_linker(&self.id(), &linker_cmd, &project.project_dir())?;

            self.toolchain_config
                .setup_tool("AR", &self.toolchain_config.naked_executable("llvm-ar"))?;
        }

        if self.ndk_major_version >= 17 {
            // bindgen need this to use the proper imports
            set_env("DINGHY_BUILD_LIBCLANG_PATH", &self.libclang_path)
        }

        if std::env::var("ANDROID_NDK").is_err() {
            set_env("ANDROID_NDK", self.ndk_path.canonicalize()?)
        }

        if std::env::var("ANDROID_NDK_HOME").is_err() {
            set_env("ANDROID_NDK_HOME", self.ndk_path.canonicalize()?)
        }

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
        if self.ndk_major_version >= 23 {
            build.runnable = platform::strip_runnable(
                &build.runnable,
                Command::new(self.toolchain_config.naked_executable("llvm-strip")),
            )?;
            Ok(())
        } else {
            self.regular_platform.strip(build)
        }
    }

    fn sysroot(&self) -> anyhow::Result<Option<PathBuf>> {
        self.regular_platform.sysroot()
    }
}
