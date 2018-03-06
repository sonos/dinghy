use std::env;
use std::path::Path;
use super::Result;

pub fn is_cross_compiling() -> Result<bool> {
    Ok(env::var("TARGET")? != env::var("HOST")?)
}

pub fn include_path<P: AsRef<Path>>(lib_dir_path: P) -> Result<()> {
    println!("cargo:rustc-link-lib={}", lib_dir_path.as_ref().display());
    Ok(())
}

pub fn link_dylib<P: AsRef<Path>>(lib_name: &str, lib_dir_path: P) -> Result<()> {
    println!("cargo:rustc-link-search={}", lib_dir_path.as_ref().display());
    println!("cargo:rustc-link-lib=dylib={}", lib_name);
    Ok(())
}

pub fn link_lib<P: AsRef<Path>>(lib_name: &str, lib_dir_path: P) -> Result<()> {
    println!("cargo:rustc-link-search={}", lib_dir_path.as_ref().display());
    println!("cargo:rustc-link-lib={}", lib_name);
    Ok(())
}

pub fn link_system_dylib(lib_name: &str) -> Result<()> {
    println!("cargo:rustc-link-lib=dylib={}", lib_name);
    Ok(())
}

pub fn link_system_lib(lib_name: &str) -> Result<()> {
    println!("cargo:rustc-link-lib={}", lib_name);
    Ok(())
}

pub fn rerun_if_changed<P: AsRef<Path>>(filepath: P) {
    println!("cargo:rerun-if-changed={}", filepath.as_ref().display());
}
