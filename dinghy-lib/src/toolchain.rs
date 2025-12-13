use crate::errors::*;
use crate::SetupArgs;
use dinghy_build::build_env::append_path_to_env;
use dinghy_build::build_env::append_path_to_target_env;
use dinghy_build::build_env::envify;
use dinghy_build::build_env::set_env;
use dinghy_build::build_env::set_target_env;
use itertools::Itertools;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::{fs, path};
use walkdir::WalkDir;

#[cfg(not(target_os = "windows"))]
static GLOB_ARGS: &str = r#""$@""#;
#[cfg(target_os = "windows")]
static GLOB_ARGS: &str = r#"%*"#;

#[derive(Clone, Debug)]
pub struct Toolchain {
    pub rustc_triple: String,
}

impl Toolchain {
    pub fn setup_tool(&self, var: &str, exe: &str) -> Result<()> {
        set_env(format!("TARGET_{}", var), exe);
        set_env(format!("{}_{}", var, self.rustc_triple), exe);
        Ok(())
    }

    pub fn setup_cc(&self, _id: &str, compiler_command: &str) -> Result<()> {
        set_env("TARGET_CC", compiler_command);
        set_env(format!("CC_{}", self.rustc_triple), compiler_command);
        Ok(())
    }

    pub fn setup_linker_raw(&self, linker: &str) {
        set_env(
            format!("CARGO_TARGET_{}_LINKER", envify(self.rustc_triple.as_str())),
            linker,
        );
    }

    pub fn setup_linker<P: AsRef<path::Path>>(
        &self,
        id: &str,
        linker_command: &str,
        workspace_root: P,
    ) -> Result<()> {
        let shim = create_shim(
            workspace_root,
            &self.rustc_triple,
            id,
            "linker",
            format!("{} {}", linker_command, GLOB_ARGS).as_str(),
        )?;
        set_env(
            format!("CARGO_TARGET_{}_LINKER", envify(self.rustc_triple.as_str())).as_str(),
            shim,
        );
        Ok(())
    }

    pub fn setup_pkg_config(&self) -> Result<()> {
        set_env("PKG_CONFIG_ALLOW_CROSS", "1");
        set_target_env("PKG_CONFIG_LIBPATH", Some(&self.rustc_triple), "");
        Ok(())
    }

    pub fn setup_runner(&self, platform_id: &str, setup_args: &SetupArgs) -> Result<()> {
        set_env(
            format!("CARGO_TARGET_{}_RUNNER", envify(self.rustc_triple.as_str())).as_str(),
            setup_args.get_runner_command(platform_id),
        );
        Ok(())
    }

    pub fn setup_target(&self) -> Result<()> {
        set_env("CARGO_BUILD_TARGET", &self.rustc_triple);
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ToolchainConfig {
    pub bin_dir: PathBuf,
    pub root: PathBuf,
    pub rustc_triple: String,
    pub sysroot: Option<PathBuf>,
    pub cc: String,
    pub cxx: String,
    pub binutils_prefix: String,
    pub cc_prefix: String,
}

impl ToolchainConfig {
    pub fn cc_executable(&self, name_without_triple: &str) -> String {
        self.bin_dir
            .join(format!("{}-{}", self.cc_prefix, name_without_triple))
            .to_string_lossy()
            .to_string()
    }

    pub fn binutils_executable(&self, name_without_triple: &str) -> String {
        self.bin_dir
            .join(format!("{}-{}", self.binutils_prefix, name_without_triple))
            .to_string_lossy()
            .to_string()
    }

    pub fn naked_executable(&self, name: &str) -> String {
        self.bin_dir.join(name).to_string_lossy().to_string()
    }

    pub fn setup_pkg_config(&self) -> Result<()> {
        self.as_toolchain().setup_pkg_config()?;

        if self.root.parent().is_some() {
            append_path_to_target_env(
                "PKG_CONFIG_LIBDIR",
                Some(&self.rustc_triple),
                WalkDir::new(self.root.to_string_lossy().as_ref())
                    .into_iter()
                    .filter_map(|e| e.ok()) // Ignore unreadable files, maybe could warn...
                    .filter(|e| e.file_name() == "pkgconfig" && e.file_type().is_dir())
                    .map(|e| e.path().to_string_lossy().into_owned())
                    .join(":"),
            );
        }

        if let Some(sr) = &self.sysroot {
            set_target_env("PKG_CONFIG_SYSROOT_DIR", Some(&self.rustc_triple), &sr);
        }
        Ok(())
    }

    pub fn setup_sysroot(&self) {
        if let Some(sr) = &self.sysroot {
            set_env("TARGET_SYSROOT", sr);
        }
    }

    pub fn setup_tool(&self, var: &str, command: &str) -> Result<()> {
        self.as_toolchain().setup_tool(var, command)
    }

    pub fn setup_cc(&self, id: &str, compiler_command: &str) -> Result<()> {
        self.as_toolchain().setup_cc(id, compiler_command)
    }

    pub fn generate_linker_command(&self, setup_args: &SetupArgs) -> String {
        let mut linker_cmd = self.cc_executable(&*self.cc);
        linker_cmd.push_str(" ");
        if setup_args.verbosity > 0 {
            linker_cmd.push_str("-Wl,--verbose -v")
        }
        if let Some(sr) = &self.sysroot {
            linker_cmd.push_str(&format!(" --sysroot {}", sr.display()));
        }
        for forced_overlay in &setup_args.forced_overlays {
            linker_cmd.push_str(" -l");
            linker_cmd.push_str(&forced_overlay);
            // TODO Add -L
        }

        linker_cmd
    }

    pub fn setup_linker_raw(&self, linker: &str) {
        self.as_toolchain().setup_linker_raw(linker)
    }

    pub fn setup_linker<P: AsRef<path::Path>>(
        &self,
        id: &str,
        linker_command: &str,
        workspace_root: P,
    ) -> Result<()> {
        self.as_toolchain()
            .setup_linker(id, linker_command, workspace_root)
    }

    pub fn setup_runner(&self, platform_id: &str, setup_args: &SetupArgs) -> Result<()> {
        self.as_toolchain().setup_runner(platform_id, setup_args)
    }

    pub fn setup_target(&self) -> Result<()> {
        self.as_toolchain().setup_target()
    }

    pub fn shim_executables<P: AsRef<path::Path>>(
        &self,
        id: &str,
        workspace_root: P,
    ) -> Result<()> {
        let workspace_root = workspace_root.as_ref();
        let shims_path = workspace_root
            .join("target")
            .join(&self.rustc_triple)
            .join(id);

        for exe in self.bin_dir.read_dir()? {
            let exe = exe?;
            let exe_file_name = exe.file_name();
            let exe_path = exe.path();
            let exe_path = exe_path.to_string_lossy();

            let rustified_exe = &exe_file_name
                .to_string_lossy()
                .replace(self.binutils_prefix.as_str(), self.rustc_triple.as_str())
                .replace(self.cc_prefix.as_str(), self.rustc_triple.as_str());
            log::trace!("Shim {} -> {}", exe_path, rustified_exe);
            create_shim(
                workspace_root,
                self.rustc_triple.as_str(),
                id,
                rustified_exe,
                &format!("{} {}", exe_path, GLOB_ARGS),
            )?;
        }
        append_path_to_env("PATH", shims_path.to_string_lossy().as_ref());
        Ok(())
    }

    fn as_toolchain(&self) -> Toolchain {
        Toolchain {
            rustc_triple: self.rustc_triple.clone(),
        }
    }
}

pub fn create_shim<P: AsRef<path::Path>>(
    root: P,
    rustc_triple: &str,
    id: &str,
    name: &str,
    shell: &str,
) -> Result<PathBuf> {
    let target_shim_path = root.as_ref().join("target").join(rustc_triple).join(id);
    fs::create_dir_all(&target_shim_path)?;
    let mut shim = target_shim_path.join(name);
    if cfg!(target_os = "windows") {
        shim.set_extension("bat");
    };
    let mut linker_shim = fs::File::create(&shim)?;
    if !cfg!(target_os = "windows") {
        writeln!(linker_shim, "#!/bin/sh")?;
    }
    linker_shim.write_all(shell.as_bytes())?;
    writeln!(linker_shim, "\n")?;
    #[cfg(unix)]
    fs::set_permissions(&shim, PermissionsExt::from_mode(0o777))?;
    Ok(shim)
}
