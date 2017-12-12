use std::{env, path};

use {Result, Platform};

#[derive(Debug)]
pub struct RegularPlatform {
    id: String,
    root: path::PathBuf,
    bin: path::PathBuf,
    rustc_triple: String,
    tc_triple: String,
    sysroot: String,
}

impl RegularPlatform {
    pub fn new<P: AsRef<path::Path>>(id:String, rustc_triple:String, toolchain: P) -> Result<Box<Platform>> {
        let mut bin: Option<path::PathBuf> = None;
        let mut prefix: Option<String> = None;
        for file in toolchain.as_ref().join("bin").read_dir()? {
            let file = file?;
            if file.file_name().to_string_lossy().ends_with("-gcc")
                || file.file_name().to_string_lossy().ends_with("-gcc.exe")
            {
                bin = Some(toolchain.as_ref().join("bin"));
                prefix = Some(
                    file.file_name()
                        .to_string_lossy()
                        .replace(".exe", "")
                        .replace("-gcc", ""),
                );
                break;
            }
        }
        let bin = bin.ok_or("no bin/*-gcc found in toolchain")?;
        let tc_triple = prefix.ok_or("no gcc in toolchain")?.to_string();
        let sysroot = sysroot_in_toolchain(&toolchain)?;
        Ok(Box::new(RegularPlatform {
            id,
            root: toolchain.as_ref().into(),
            bin,
            rustc_triple,
            tc_triple,
            sysroot,
        }))
    }

    fn binary(&self, name: &str) -> String {
        self.bin
            .join(format!("{}-{}", self.tc_triple, name))
            .to_string_lossy()
            .into()
    }
}

impl ::std::fmt::Display for RegularPlatform {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(f, "{:?}", self.root)
    }
}

impl Platform for RegularPlatform {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn rustc_triple(&self) -> Result<String> {
        Ok(self.rustc_triple.to_string())
    }
    fn cc_command(&self) -> Result<String> {
        Ok(format!("{} {}", self.binary("gcc"), ::shim::GLOB_ARGS))
    }
    fn linker_command(&self) -> Result<String> {
        Ok(format!(
            "{} --sysroot {} {}",
            self.binary("gcc"),
            self.sysroot,
            ::shim::GLOB_ARGS
        ))
    }
    fn setup_more_env(&self) -> Result<()> {
        let wd_path = ::cargo::util::important_paths::find_root_manifest_for_wd(None, &env::current_dir()?)?;
        let root = wd_path.parent().ok_or("building at / ?")?;
        let path = env::var("PATH").unwrap();
        let shims_path = root.join("target").join(&self.rustc_triple).join(&self.id);
        env::set_var("PATH", format!("{}:{}", path, shims_path.to_string_lossy()));
        for exe in self.bin.read_dir()? {
            let exe = exe?;
            let rustified_exe = &exe.file_name().to_string_lossy().replace(&self.tc_triple, &self.rustc_triple);
            println!("toolchain: {} -> {}", exe.path().to_string_lossy(), rustified_exe);
            ::shim::create_shim(root, &self.rustc_triple, &self.id, rustified_exe, &format!("{} {}", exe.path().to_string_lossy(), ::shim::GLOB_ARGS))?;
        }
        env::set_var("TARGET_SYSROOT", &self.sysroot);
        env::set_var("TARGET_AR", &self.binary("ar"));
        Ok(())
    }
}

fn sysroot_in_toolchain<P: AsRef<path::Path>>(p: P) -> Result<String> {
    let immediate = p.as_ref().join("sysroot");
    if immediate.is_dir() {
        let sysroot = immediate.to_str().ok_or("sysroot is not utf-8")?;
        return Ok(sysroot.into());
    }
    for subdir in p.as_ref().read_dir()? {
        let subdir = subdir?;
        let maybe = subdir.path().join("sysroot");
        if maybe.is_dir() {
            let sysroot = maybe.to_str().ok_or("sysroot is not utf-8")?;
            return Ok(sysroot.into());
        }
    }
    Err(format!("no sysroot found in toolchain {:?}", p.as_ref()))?
}
