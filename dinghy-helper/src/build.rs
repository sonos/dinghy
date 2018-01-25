use std::env;
use toolchain::sysroot_path;
use utils::path_to_str;
use super::Result;

pub fn is_cross_compiling() -> Result<bool> {
    Ok(env::var("TARGET")? != env::var("HOST")?)
}

pub fn link_system_lib(lib_name: &str) -> Result<()> {
    println!("cargo:rustc-link-lib={}", lib_name);
    Ok(())
}
