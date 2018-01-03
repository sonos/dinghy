use std::{env, path};

use {Result, Platform};
use Device;
use std::ascii::AsciiExt;
use std::ffi::OsStr;
use itertools::Itertools;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct RegularPlatform {
    pub id: String,
    pub root: path::PathBuf,
    pub bin: path::PathBuf,
    pub rustc_triple: String,
    pub tc_triple: String,
    pub sysroot: String,
}

impl RegularPlatform {
    pub fn new<P: AsRef<path::Path>>(id:String, rustc_triple:String, toolchain: P) -> Result<Box<Platform>> {
        let toolchain_path = toolchain.as_ref();
        let mut bin: Option<path::PathBuf> = None;
        let mut prefix: Option<String> = None;

        for file in toolchain_path.join("bin").read_dir().map_err(|_| format!("Couldn't find toolchain directory {}", toolchain_path.display()))? {
            let file = file?;
            if file.file_name().to_string_lossy().ends_with("-gcc")
                || file.file_name().to_string_lossy().ends_with("-gcc.exe")
            {
                bin = Some(toolchain_path.join("bin"));
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
        let sysroot = sysroot_in_toolchain(&toolchain_path)?;
        Ok(Box::new(RegularPlatform {
            id,
            root: toolchain_path.into(),
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
        fn set_env<K: AsRef<OsStr>, V: AsRef<OsStr>>(k: K, v: V) {
            info!("Setting environment variable {:?}='{:?}'", k.as_ref(), v.as_ref());
            println!("Setting environment variable {:?}='{:?}'", k.as_ref(), v.as_ref());
            env::set_var(k, v);
        }

        fn envify(name: &str) -> String {
            name.chars().map(|c| c.to_ascii_uppercase()).map(|c| {
                if c == '-' {'_'} else {c}
            }).collect()
        }

        let wd_path = ::cargo::util::important_paths::find_root_manifest_for_wd(None, &env::current_dir()?)?;
        let root = wd_path.parent().ok_or("building at / ?")?;
        let path = env::var("PATH").unwrap();
        let shims_path = root.join("target").join(&self.rustc_triple).join(&self.id);

        set_env("PATH", format!("{}:{}", path, shims_path.to_string_lossy()));
        for exe in self.bin.read_dir()? {
            let exe = exe?;
            let rustified_exe = &exe.file_name().to_string_lossy().replace(&self.tc_triple, &self.rustc_triple);
            println!("toolchain: {} -> {}", exe.path().to_string_lossy(), rustified_exe);
            info!("toolchain: {} -> {}", exe.path().to_string_lossy(), rustified_exe);
            ::shim::create_shim(root, &self.rustc_triple, &self.id, rustified_exe, &format!("{} {}", exe.path().to_string_lossy(), ::shim::GLOB_ARGS))?;
        }
        set_env("TARGET_SYSROOT", &self.sysroot.clone());
        set_env("TARGET_AR", &self.binary("ar"));
        set_env("PKG_CONFIG_ALLOW_CROSS", "1");
        set_env(format!("{}_PKG_CONFIG_LIBDIR", envify(self.rustc_triple()?.as_str())), WalkDir::new(self.root.to_string_lossy().as_ref())
            .into_iter()
            .filter_map(|e| e.ok()) // Ignore unreadable files, maybe could warn...
            .filter(|e| e.file_name() == "pkgconfig" && e.file_type().is_dir())
            .map(|e| e.path().to_string_lossy().into_owned())
            .join(":"));
        set_env(format!("{}_PKG_CONFIG_SYSROOT_DIR", envify(self.rustc_triple()?.as_str())), &self.sysroot.clone());
        Ok(())
    }

    fn is_compatible_with(&self, device: &Device) -> bool {
        device.is_compatible_with_regular_platform(self)
    }
}

fn sysroot_in_toolchain<P: AsRef<path::Path>>(toolchain_path: P) -> Result<String> {
    let toolchain = toolchain_path.as_ref();
    let immediate = toolchain.join("sysroot");
    if immediate.is_dir() {
        let sysroot = immediate.to_str().ok_or("sysroot is not utf-8")?;
        return Ok(sysroot.into());
    }
    for subdir in toolchain.read_dir()? {
        let subdir = subdir?;
        let maybe = subdir.path().join("sysroot");
        if maybe.is_dir() {
            let sysroot = maybe.to_str().ok_or("sysroot is not utf-8")?;
            return Ok(sysroot.into());
        }
    }
    Err(format!("no sysroot found in toolchain {:?}", toolchain))?
}
