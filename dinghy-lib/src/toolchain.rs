use dinghy_build::build_env::append_path_to_target_env; // FIXME
use dinghy_build::build_env::append_path_to_env;
use dinghy_build::build_env::envify;
use dinghy_build::build_env::set_env;
use dinghy_build::build_env::target_key_from_triple;
use errors::*;
use itertools::Itertools;
use std::{fs, path};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use walkdir::WalkDir;
use utils::{ GLOB_ARGS, create_shim, project_root };

#[derive(Clone, Debug)]
pub struct Toolchain {
    pub rustc_triple: String,
}

impl Toolchain {
    pub fn setup_tool(&self, var: &str, exe: &str, env: &mut HashMap<String, Option<String>>) -> Result<()> {
        set_env(format!("TARGET_{}", var), exe);
        set_env(format!("{}_{}", var, self.rustc_triple), exe);
        Ok(())
    }

    pub fn setup_cc(&self, _id: &str, compiler_command: &str, env:&mut HashMap<String,Option<String>>) -> Result<()> {
        self.setup_tool("CC", compiler_command, env)
    }

    pub fn setup_linker(&self, id: &str, linker_command: &str, env:&mut HashMap<String,Option<String>>) -> Result<()> {
        let shim = create_shim(project_root()?, "toolchain", &self.rustc_triple, id, "linker", format!("{} {}", linker_command, GLOB_ARGS).as_str())?;
        env.insert(
            format!("CARGO_TARGET_{}_LINKER", envify(&self.rustc_triple)),
            Some(shim.to_string_lossy().to_string()));
        Ok(())
    }

    pub fn setup_pkg_config(&self, env: &mut HashMap<String, Option<String>>) -> Result<()> {
        env.insert("PKG_CONFIG_ALLOW_CROSS".into(), Some("1".into()));
        env.insert(target_key_from_triple("PKG_CONFIG_LIBPATH", Some(&self.rustc_triple)).to_string_lossy().to_string(), None);
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ToolchainConfig {
    pub bin: PathBuf,
    pub root: PathBuf,
    pub rustc_triple: String,
    pub sysroot: PathBuf,
    pub toolchain_triple: String,
}

impl ToolchainConfig {
    pub fn executable(&self, name_without_triple: &str) -> String {
        self.bin
            .join(format!("{}-{}", self.toolchain_triple, name_without_triple))
            .to_string_lossy()
            .to_string()
    }

    pub fn setup_pkg_config(&self, env: &mut HashMap<String, Option<String>>) -> Result<()> {
        self.as_toolchain().setup_pkg_config(env)?;

        append_path_to_target_env("PKG_CONFIG_LIBDIR",
                                  Some(&self.rustc_triple),
                                  WalkDir::new(self.root.to_string_lossy().as_ref())
                                      .into_iter()
                                      .filter_map(|e| e.ok()) // Ignore unreadable files, maybe could warn...
                                      .filter(|e| e.file_name() == "pkgconfig" && e.file_type().is_dir())
                                      .map(|e| e.path().to_string_lossy().into_owned())
                                      .join(":"));
        env.insert(
            target_key_from_triple("PKG_CONFIG_SYSROOT_DIR", Some(&self.rustc_triple)).to_string_lossy().to_string(),
            Some(self.sysroot.to_string_lossy().to_string()));
        Ok(())
    }

    pub fn setup_sysroot(&self) {
        set_env("TARGET_SYSROOT", &self.sysroot);
    }

    pub fn setup_tool(&self, var: &str, command: &str, env: &mut HashMap<String,Option<String>>) -> Result<()> {
        self.as_toolchain().setup_tool(var, command, env)
    }

    pub fn setup_cc(&self, id: &str, compiler_command: &str, env: &mut HashMap<String,Option<String>>) -> Result<()> {
        self.as_toolchain().setup_cc(id, compiler_command, env)
    }

    pub fn setup_linker(&self, id: &str, linker_command: &str, env: &mut HashMap<String,Option<String>>) -> Result<()> {
        self.as_toolchain().setup_linker(id, linker_command, env)
    }

    pub fn shim_executables(&self, id: &str) -> Result<()> {
        let workspace = ::cargo_metadata::metadata(None)?;
        let root:PathBuf = workspace.workspace_root.into();
        let shims_path = root.join("target").join(self.rustc_triple.as_str()).join(id);

        for exe in self.bin.read_dir()? {
            let exe = exe?;
            let exe_file_name = exe.file_name();
            let exe_path = exe.path();
            let exe_path = exe_path.to_string_lossy(); // Rust and paths = 💩💩💩

            let rustified_exe = &exe_file_name.to_string_lossy().replace(self.toolchain_triple.as_str(),
                                                                         self.rustc_triple.as_str());
            trace!("Shim {} -> {}", exe_path, rustified_exe);
            create_shim(&root,
                        "toolchain",
                        self.rustc_triple.as_str(),
                        id,
                        rustified_exe,
                        &format!("{} {}", exe_path, GLOB_ARGS))?;
        }
        append_path_to_env("PATH", shims_path.to_string_lossy().as_ref());
        Ok(())
    }

    fn as_toolchain(&self) -> Toolchain {
        Toolchain { rustc_triple: self.rustc_triple.clone() }
    }
}

