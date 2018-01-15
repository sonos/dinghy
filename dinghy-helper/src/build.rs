use std::env;
use toolchain::sysroot_path;
use utils::path_to_str;
use super::Result;

pub fn is_cross_compiling() -> Result<bool> {
    Ok(env::var("TARGET")? != env::var("HOST")?)
}

pub fn link_lib(lib_name: &str) -> Result<()> {
    if is_cross_compiling()? {
        let lib_dir = sysroot_path()?.join("usr").join("lib");
        println!("cargo:rustc-link-search={}", path_to_str(&lib_dir)?);
    }
    println!("cargo:rustc-link-lib={}", lib_name);
    Ok(())
}
