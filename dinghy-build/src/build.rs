use super::Result;
///! Helpers functions to output `cargo:` lines  suitable for build.rs output.
use std::env;
use std::path::Path;

/// Find out if we are cross-compiling.
pub fn is_cross_compiling() -> Result<bool> {
    Ok(env::var("TARGET")? != env::var("HOST")?)
}

/// Adds a `cargo:rustc-link-lib=` line.
pub fn include_path<P: AsRef<Path>>(lib_dir_path: P) -> Result<()> {
    println!("cargo:rustc-link-lib={}", lib_dir_path.as_ref().display());
    Ok(())
}

/// Adds a `cargo:rustc-link-search=` and `cargo:rustc-link-lib=static=` line.
pub fn link_static<P: AsRef<Path>>(lib_name: &str, lib_dir_path: P) -> Result<()> {
    println!(
        "cargo:rustc-link-search={}",
        lib_dir_path.as_ref().display()
    );
    println!("cargo:rustc-link-lib=static={}", lib_name);
    Ok(())
}

/// Adds a `cargo:rustc-link-search=` and `cargo:rustc-link-lib=dylib=` line.
pub fn link_dylib<P: AsRef<Path>>(lib_name: &str, lib_dir_path: P) -> Result<()> {
    println!(
        "cargo:rustc-link-search={}",
        lib_dir_path.as_ref().display()
    );
    println!("cargo:rustc-link-lib=dylib={}", lib_name);
    Ok(())
}

/// Adds a `cargo:rustc-link-search` and `cargo:rustc-link-lib=` line.
pub fn link_lib<P: AsRef<Path>>(lib_name: &str, lib_dir_path: P) -> Result<()> {
    println!(
        "cargo:rustc-link-search={}",
        lib_dir_path.as_ref().display()
    );
    println!("cargo:rustc-link-lib={}", lib_name);
    Ok(())
}

/// Adds a `cargo:rustc-link-lib=dylib=` line.
pub fn link_system_dylib(lib_name: &str) -> Result<()> {
    println!("cargo:rustc-link-lib=dylib={}", lib_name);
    Ok(())
}

/// Adds a `cargo:rustc-link-lib=` line.
pub fn link_system_lib(lib_name: &str) -> Result<()> {
    println!("cargo:rustc-link-lib={}", lib_name);
    Ok(())
}

/// Adds a `cargo:rerun-if-changed=` line.
pub fn rerun_if_changed<P: AsRef<Path>>(filepath: P) {
    println!("cargo:rerun-if-changed={}", filepath.as_ref().display());
}
