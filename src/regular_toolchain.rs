use std::{ env, path };

use { Result, Toolchain };

#[derive(Debug)]
pub struct RegularToolchain {
    bin: path::PathBuf,
    cc: String,
    sysroot: String,
}

impl RegularToolchain {
    pub fn new<P: AsRef<path::Path>>(toolchain:P) -> Result<Box<Toolchain>> {
        let mut bin: Option<path::PathBuf> = None;
        let mut cc: Option<path::PathBuf> = None;
        for file in toolchain.as_ref().join("bin").read_dir()? {
            let file = file?;
            if file.file_name().to_string_lossy().ends_with("-gcc")
                || file.file_name().to_string_lossy().ends_with("-gcc.exe")
            {
                bin = Some(toolchain.as_ref().join("bin"));
                cc = Some(file.path());
                break;
            }
        }
        let bin = bin.ok_or("no bin/*-gcc found in toolchain")?;
        let cc = cc.ok_or("no bin/*-gcc found in toolchain")?;
        let cc = cc.to_str().ok_or("path is not utf-8")?.to_string();
        let sysroot = sysroot_in_toolchain(toolchain)?;
        Ok(Box::new(RegularToolchain { bin, cc, sysroot }))
    }
}

impl Toolchain for RegularToolchain {
    fn cc_command(&self, _target: &str) -> Result<String> {
        Ok(format!("{} {}", self.cc, ::shim::GLOB_ARGS))
    }
    fn linker_command(&self, _target: &str) -> Result<String> {
        Ok(format!(
            "{} --sysroot {} {}",
            self.cc,
            self.sysroot,
            ::shim::GLOB_ARGS
        ))
    }
    fn setup_more_env(&self, _target: &str) -> Result<()> {
        env::set_var("TARGET_SYSROOT", &self.sysroot);
        Ok(())
    }
}

fn sysroot_in_toolchain<P: AsRef<path::Path>>(p: P) -> Result<String> {
    let immediate = p.as_ref().join("sysroot");
    if immediate.is_dir() {
        let sysroot = immediate.to_str().ok_or("sysroot is not utf-8")?;
        return Ok(sysroot.into())
    }
    for subdir in p.as_ref().read_dir()? {
        let subdir = subdir?;
        let maybe = subdir.path().join("sysroot");
        if maybe.is_dir() {
            let sysroot = maybe.to_str().ok_or("sysroot is not utf-8")?;
            return Ok(sysroot.into())
        }
    }
    Err(format!("no sysroot found in toolchain {:?}", p.as_ref()))?
}
