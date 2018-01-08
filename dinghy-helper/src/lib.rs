extern crate bindgen;
#[macro_use]
extern crate error_chain;
extern crate gcc;
#[macro_use]
extern crate log;

use std::env;
use std::env::current_dir;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        EnvVar(::std::env::VarError);
        StringFromUtf8(::std::string::FromUtf8Error);
    }
}

pub mod os_env;

pub struct DinghyHelper {}

fn path_as_str(path: &PathBuf) -> Result<&str> {
    Ok(path.to_str()
        .ok_or(format!("Not a valid UTF-8 path ({})", path.display()))?)
}

impl DinghyHelper {
    pub fn bindgen_with_cross_compilation() -> Result<bindgen::Builder> {
        Ok(bindgen::Builder::default()
            .clang_arg("--verbose")
            .detect_toolchain()?
            .include_gcc_system_headers()?)
    }

    pub fn link_lib(lib_name: &str) -> Result<()> {
        if DinghyHelper::is_cross_compiling()? {
            let lib_dir = DinghyHelper::env_sysroot()?.join("usr").join("lib");
            println!("cargo:rustc-link-search={}", path_as_str(&lib_dir)?);
        }
        println!("cargo:rustc-link-lib={}", lib_name);
        Ok(())
    }

    pub fn is_cross_compiling() -> Result<bool> {
        Ok(env::var("TARGET")? != env::var("HOST")?)
    }

    pub fn target_env(var_base: &str) -> Result<String> {
        if let Ok(target) = env::var("TARGET") {
            let is_host = env::var("HOST")? == target;
            DinghyHelper::target_env_from_triple(var_base, target.as_str(), is_host)
        } else {
            DinghyHelper::env_var_with_metadata(var_base)
        }
    }

    pub fn target_env_from_triple(var_base: &str, triple: &str, host: bool) -> Result<String> {
        DinghyHelper::env_var_with_metadata(&format!("{}_{}", var_base, triple))
            .or_else(|_| DinghyHelper::env_var_with_metadata(&format!("{}_{}", var_base, triple.replace("-", "_"))))
            .or_else(|_| DinghyHelper::env_var_with_metadata(&format!("{}_{}", if host { "HOST" } else { "TARGET" }, var_base)))
            .or_else(|_| DinghyHelper::env_var_with_metadata(var_base))
    }

    pub fn set_env<K: AsRef<OsStr>, V: AsRef<OsStr>>(k: K, v: V) {
        info!("Setting environment variable {:?}='{:?}'", k.as_ref(), v.as_ref());
        env::set_var(k, v);
    }

    pub fn set_target_env<K: AsRef<OsStr>, V: AsRef<OsStr>>(k: K, rustc_triple: &str, v: V) {
        let mut key = OsString::new();
        key.push(k);
        key.push("_");
        key.push(rustc_triple.replace("-", "_"));
        info!("Setting target environment variable {:?}={:?}", key, v.as_ref());
        env::set_var(key, v);
    }

    pub fn append_path_target_env<K: AsRef<OsStr>, V: AsRef<OsStr>>(k: K, rustc_triple: &str, v: V) {
        let mut target_key = OsString::new();
        target_key.push(k);
        target_key.push("_");
        target_key.push(rustc_triple.replace("-", "_"));

        DinghyHelper::append_path_to_env(target_key, v.as_ref())
    }

    pub fn append_path_to_env<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
        info!("Appending {:?} to environment variable '{:?}'", value.as_ref(), key.as_ref());
        let mut formatted_value = OsString::new();
        if let Ok(initial_value) = env::var(key.as_ref()) {
            formatted_value.push(initial_value);
            formatted_value.push(":");
        }
        formatted_value.push(value);
        DinghyHelper::set_env(key.as_ref(), formatted_value);
    }

    pub fn envify(name: &str) -> String {
        // Same as name.replace("-", "_").to_uppercase()
        name.chars()
            .map(|c| c.to_ascii_uppercase())
            .map(|c| { if c == '-' { '_' } else { c } })
            .collect()
    }

    fn env_var_with_metadata(name: &str) -> Result<String> {
        println!("cargo:rerun-if-env-changed={}", name);
        Ok(env::var(name)?)
    }

    fn env_sysroot() -> Result<PathBuf> {
        env::var_os("TARGET_SYSROOT").map(PathBuf::from).chain_err(|| "You must either define a TARGET_SYSROOT or use Dinghy to build.")
    }
}

pub trait CommandExt {
    fn with_pkgconfig(&mut self) -> Result<&mut Command>;

    fn with_toolchain(&mut self) -> Result<&mut Command>;
}

impl CommandExt for Command {
    fn with_pkgconfig(&mut self) -> Result<&mut Command> {
        if DinghyHelper::is_cross_compiling()? {
            if let Ok(value) = DinghyHelper::target_env("PKG_CONFIG_PATH") {
                self.env("PKG_CONFIG_PATH", value);
            }
            if let Ok(value) = DinghyHelper::target_env("PKG_CONFIG_LIBDIR") {
                self.env("PKG_CONFIG_LIBDIR", value);
            }
            if let Ok(value) = DinghyHelper::target_env("PKG_CONFIG_SYSROOT_DIR") {
                self.env("PKG_CONFIG_SYSROOT_DIR", value);
            }
        }
        Ok(self)
    }

    fn with_toolchain(&mut self) -> Result<&mut Command> {
        if DinghyHelper::is_cross_compiling()? {
            if let Ok(target) = env::var("TARGET") {
                self.arg(format!("--host={}", target));
            }
            if let Ok(cc) = env::var("TARGET_CC") {
                self.arg(format!("CC={}", cc));
            }
            if let Ok(ar) = env::var("TARGET_AR") {
                self.arg(format!("AR={}", ar));
            }
            if let Ok(sysroot) = env::var("TARGET_SYSROOT") {
                self.arg(format!("--with-sysroot={}", &sysroot));
            }
//            --define-variable=prefix=/foo
        }
        Ok(self)
    }
}

pub trait BindGenBuilderExt {
    fn detect_toolchain(self) -> Result<bindgen::Builder>;

    fn generate_default_binding(self) -> Result<()>;

    fn header_in_current_dir(self, header_file_name: &str) -> Result<bindgen::Builder>;

    fn include_gcc_system_headers(self) -> Result<bindgen::Builder>;
}

impl BindGenBuilderExt for bindgen::Builder {
    fn detect_toolchain(self) -> Result<bindgen::Builder> {
        if DinghyHelper::is_cross_compiling()? {
            let target = env::var("TARGET")?;
            Ok(self
                .clang_arg(format!("--sysroot={}", path_as_str(&DinghyHelper::env_sysroot()?)?))
                .clang_arg(format!("--target={}", target)))
        } else {
            Ok(self)
        }
    }

    fn generate_default_binding(self) -> Result<()> {
        let out_path = env::var("OUT_DIR").map(PathBuf::from)?.join("bindings.rs");
        Ok(self.generate()
            .expect("Unable to generate bindings")
            .write_to_file(out_path)?)
    }

    fn header_in_current_dir(self, header_file_name: &str) -> Result<bindgen::Builder> {
        let header_path = current_dir().map(PathBuf::from)?.join(header_file_name);
        Ok(self.header(header_path.to_str()
            .ok_or(format!("Not a valid UTF-8 path ({})", header_path.display()))?))
    }

    fn include_gcc_system_headers(self) -> Result<bindgen::Builder> {
        if DinghyHelper::is_cross_compiling()? {
            // Add a path to the private headers for the target compiler. Borderline,
            // as we are likely using a gcc header with clang frontend.
            let path = gcc::Build::new()
                .get_compiler()
                .to_command()
                .arg("--print-file-name=include")
                .output()
                .chain_err(|| "Couldn't find target GCC executable.")
                .and_then(|output| if output.status.success() {
                    Ok(String::from_utf8(output.stdout)?)
                } else {
                    bail!("Couldn't determine target GCC include dir.")
                })?;

            Ok(self
                .clang_arg("-isystem")
                .clang_arg(path.trim()))
        } else {
            Ok(self)
        }
    }
}
