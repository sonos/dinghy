use std::{env, fs, path};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use errors::*;

use cargo::util::important_paths::find_root_manifest_for_wd;

#[cfg(not(target_os = "windows"))]
pub static GLOB_ARGS: &str = r#""$@""#;
#[cfg(target_os = "windows")]
pub static GLOB_ARGS: &str = r#"%*"#;

pub fn setup_shim(rustc_triple: &str, id: &str, var: &str, name: &str, shell: &str) -> Result<()> {
    debug!("  * shim for {}: {}", name, shell);
    let wd_path = find_root_manifest_for_wd(None, &env::current_dir()?)?;
    let root = wd_path.parent().ok_or("building at / ?")?;
    let shim = create_shim(&root, rustc_triple, id, name, shell)?;
    env::set_var(var, shim);
    Ok(())
}

pub fn create_shim<P: AsRef<path::Path>>(
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
