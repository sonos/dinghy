use std::env;
use super::Result;

pub fn is_cross_compiling() -> Result<bool> {
    Ok(env::var("TARGET")? != env::var("HOST")?)
}

pub fn link_system_lib(lib_name: &str) -> Result<()> {
    println!("cargo:rustc-link-lib={}", lib_name);
    Ok(())
}
