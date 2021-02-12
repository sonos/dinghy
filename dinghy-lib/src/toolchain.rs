use crate::errors::*;
use cargo::util::important_paths::find_root_manifest_for_wd;
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
use std::{env, fs, path};
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

    pub fn setup_linker(&self, id: &str, linker_command: &str) -> Result<()> {
        let shim = create_shim(
            project_root()?,
            &self.rustc_triple,
            id,
            "linker",
            format!("{} {}", linker_command, GLOB_ARGS).as_str(),
        )?;
        set_env(
            format!("CARGO_TARGET_{}_LINKER", envify(self.rustc_triple.as_str())).as_str(),
            shim
        );
        Ok(())
    }

    pub fn setup_pkg_config(&self) -> Result<()> {
        set_env("PKG_CONFIG_ALLOW_CROSS", "1");
        set_target_env("PKG_CONFIG_LIBPATH", Some(&self.rustc_triple), "");
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ToolchainConfig {
    pub bin_dir: PathBuf,
    pub root: PathBuf,
    pub rustc_triple: String,
    pub sysroot: PathBuf,
    pub cc: String,
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

        set_target_env(
            "PKG_CONFIG_SYSROOT_DIR",
            Some(&self.rustc_triple),
            &self.sysroot.clone(),
        );
        Ok(())
    }

    pub fn setup_sysroot(&self) {
        set_env("TARGET_SYSROOT", &self.sysroot);
    }

    pub fn setup_tool(&self, var: &str, command: &str) -> Result<()> {
        self.as_toolchain().setup_tool(var, command)
    }

    pub fn setup_cc(&self, id: &str, compiler_command: &str) -> Result<()> {
        self.as_toolchain().setup_cc(id, compiler_command)
    }

    pub fn setup_linker(&self, id: &str, linker_command: &str) -> Result<()> {
        self.as_toolchain().setup_linker(id, linker_command)
    }

    pub fn shim_executables(&self, id: &str) -> Result<()> {
        let wd_path =
            ::cargo::util::important_paths::find_root_manifest_for_wd(&env::current_dir()?)?;
        let root = wd_path.parent().ok_or_else(|| anyhow!("building at / ?"))?;
        let shims_path = root.join("target").join(&self.rustc_triple).join(id);

        for exe in self.bin_dir.read_dir()? {
            let exe = exe?;
            let exe_file_name = exe.file_name();
            let exe_path = exe.path();
            let exe_path = exe_path.to_string_lossy();

            let rustified_exe = &exe_file_name
                .to_string_lossy()
                .replace(self.binutils_prefix.as_str(), self.rustc_triple.as_str())
                .replace(self.cc_prefix.as_str(), self.rustc_triple.as_str());
            trace!("Shim {} -> {}", exe_path, rustified_exe);
            create_shim(
                root,
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

fn create_shim<P: AsRef<path::Path>>(
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

fn project_root() -> Result<PathBuf> {
    let wd_path = find_root_manifest_for_wd(&env::current_dir()?)?;
    Ok(wd_path
        .parent()
        .ok_or_else(|| anyhow!("building at / ?"))?
        .to_path_buf())
}
