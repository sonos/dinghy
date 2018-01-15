use cargo::util::important_paths::find_root_manifest_for_wd;
use dinghy_helper::build_env::append_path_to_target_env;
use dinghy_helper::build_env::append_path_to_env;
use dinghy_helper::build_env::envify_key;
use dinghy_helper::build_env::set_env;
use dinghy_helper::build_env::set_target_env;
use errors::*;
use itertools::Itertools;
use std::{env, fs, path};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use walkdir::WalkDir;

#[cfg(not(target_os = "windows"))]
static GLOB_ARGS: &str = r#""$@""#;
#[cfg(target_os = "windows")]
static GLOB_ARGS: &str = r#"%*"#;

#[derive(Clone, Debug)]
pub struct Toolchain {
    pub rustc_triple: String,
}

#[derive(Clone, Debug)]
pub struct ToolchainConfig {
    pub bin: PathBuf,
    pub root: PathBuf,
    pub rustc_triple: String,
    pub sysroot: String,
    pub tc_triple: String,
}

impl Toolchain {
    pub fn setup_ar(&self, ar_command: &str) -> Result<()> {
        set_env("TARGET_AR", ar_command);
        Ok(())
    }

    pub fn setup_cc(&self, id: &str, compiler_command: &str) -> Result<()> {
        Ok(ToolchainConfig::setup_shim(
            self.rustc_triple.as_str(),
            id,
            "TARGET_CC",
            "cc",
            format!("{} {}", compiler_command, GLOB_ARGS).as_str())?)
    }

    pub fn setup_linker(&self, id: &str, linker_command: &str) -> Result<()> {
        Ok(ToolchainConfig::setup_shim(
            self.rustc_triple.as_str(),
            id,
            format!("CARGO_TARGET_{}_LINKER", envify_key(self.rustc_triple.as_str())).as_str(),
            "linker",
            format!("{} {}", linker_command, GLOB_ARGS).as_str())?)
    }
}

impl ToolchainConfig {
    pub fn setup_pkg_config(&self) -> Result<()> {
        set_env("PKG_CONFIG_ALLOW_CROSS", "1");

        set_target_env("PKG_CONFIG_LIBPATH",
                       self.rustc_triple.as_str(),
                       "");

        append_path_to_target_env("PKG_CONFIG_LIBDIR",
                                  self.rustc_triple.as_str(),
                                  WalkDir::new(self.root.to_string_lossy().as_ref())
                                   .into_iter()
                                   .filter_map(|e| e.ok()) // Ignore unreadable files, maybe could warn...
                                   .filter(|e| e.file_name() == "pkgconfig" && e.file_type().is_dir())
                                   .map(|e| e.path().to_string_lossy().into_owned())
                                   .join(":"));

        set_target_env("PKG_CONFIG_SYSROOT_DIR",
                       self.rustc_triple.as_str(),
                       &self.sysroot.clone());
        Ok(())
    }

    pub fn setup_sysroot(&self) {
        set_env("TARGET_SYSROOT", &self.sysroot.as_str());
    }

    pub fn shim_executables(&self, id: &str) -> Result<()> {
        let wd_path = ::cargo::util::important_paths::find_root_manifest_for_wd(None, &env::current_dir()?)?;
        let root = wd_path.parent().ok_or("building at / ?")?;
        let shims_path = root.join("target").join(self.rustc_triple.as_str()).join(id);

        for exe in self.bin.read_dir()? {
            let exe = exe?;
            let exe_file_name = exe.file_name();
            let exe_path = exe.path();
            let exe_path = exe_path.to_string_lossy(); // Rust and paths = 💩💩💩

            let rustified_exe = &exe_file_name.to_string_lossy().replace(self.tc_triple.as_str(),
                                                                         self.rustc_triple.as_str());
            trace!("Shim {} -> {}", exe_path, rustified_exe);
            ToolchainConfig::create_shim(root,
                                         self.rustc_triple.as_str(),
                                         id,
                                         rustified_exe,
                                         &format!("{} {}", exe_path, GLOB_ARGS))?;
        }
        append_path_to_env("PATH", shims_path.to_string_lossy().as_ref());
        Ok(())
    }

    pub fn setup_ar(&self, ar_command: &str) -> Result<()> {
        self.as_toolchain().setup_ar(ar_command)
    }

    pub fn setup_cc(&self, id: &str, compiler_command: &str) -> Result<()> {
        self.as_toolchain().setup_cc(id, compiler_command)
    }

    pub fn setup_linker(&self, id: &str, linker_command: &str) -> Result<()> {
        self.as_toolchain().setup_linker(id, linker_command)
    }

    fn setup_shim(rustc_triple: &str, id: &str, var: &str, name: &str, shell: &str) -> Result<()> {
        trace!("Shim {} -> {}", name, shell);
        let wd_path = find_root_manifest_for_wd(None, &env::current_dir()?)?;
        let root = wd_path.parent().ok_or("building at / ?")?;
        let shim = ToolchainConfig::create_shim(&root, rustc_triple, id, name, shell)?;
        env::set_var(var, shim);
        Ok(())
    }

    fn create_shim<P: AsRef<path::Path>>(
        root: P,
        rustc_triple: &str,
        id: &str,
        name: &str,
        shell: &str,
    ) -> Result<path::PathBuf> {
        let target_path = root.as_ref().join("target").join(rustc_triple).join(id);
        fs::create_dir_all(&target_path)?;
        let mut shim = target_path.join(name);
        if cfg!(target_os = "windows") {
            shim.set_extension("bat");
        };
        let mut linker_shim = fs::File::create(&shim)?;
        if !cfg!(target_os = "windows") {
            writeln!(linker_shim, "#!/bin/sh")?;
        }
        linker_shim.write_all(shell.as_bytes())?;
        writeln!(linker_shim, "\n")?;
        if !cfg!(target_os = "windows") {
            fs::set_permissions(&shim, PermissionsExt::from_mode(0o777))?;
        }
        Ok(shim)
    }

    pub fn executable(&self, name_without_triple: &str) -> String {
        self.bin
            .join(format!("{}-{}", self.tc_triple, name_without_triple))
            .to_string_lossy()
            .to_string()
    }

    fn as_toolchain(&self) -> Toolchain {
        Toolchain { rustc_triple: self.rustc_triple.clone() }
    }
}
