use std::env;
use std::path::PathBuf;
use super::Result;
use super::ResultExt;

pub fn sysroot_path() -> Result<PathBuf> {
    env::var_os("TARGET_SYSROOT").map(PathBuf::from).chain_err(|| "You must either define a TARGET_SYSROOT or use Dinghy to build your project.")
}
