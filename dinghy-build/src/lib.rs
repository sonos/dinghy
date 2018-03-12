//! Helpers for build.rs scripts.
//!
//! This library is meant to be used in build.rs scripts context.
//!
//! It contains a set of standalone functions that encodes some of the
//! shared wisdom and conventions across build.rs scripts, cargo, dinghy,
//! cc-rs, pkg-config-rs, bindgen, and others. It also helps providing
//! cross-compilation arguments to autotools `./configure` scripts.

extern crate bindgen;
#[macro_use]
extern crate error_chain;
extern crate gcc;
#[macro_use]
extern crate log;

pub mod build;
pub mod build_env;
pub mod utils;

use build::is_cross_compiling;
use build_env::sysroot_path;
use build_env::target_env;
use std::env;
use std::env::current_dir;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use utils::path_to_str;
use utils::path_between;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        EnvVar(::std::env::VarError);
        StringFromUtf8(::std::string::FromUtf8Error);
    }
}

/// Decorator for the std::process::Command adding a some chainable helpers.
/// 
/// Mostly useful for calling `./configure` scripts.
pub trait CommandExt {
    /// Add this argument to the commands, but only on macos.
    fn arg_for_macos<S: AsRef<OsStr>>(&mut self, arg: S) -> Result<&mut Command>;

    /// Add a `--prefix` to point to a toolchain sysroot or the /, depending on
    /// dinghy environment.
    fn configure_prefix<P: AsRef<Path>>(&mut self, path: P) -> Result<&mut Command>;

    /// Adds pkgconfig environment variables to point to an eventual cross compiling sysroot.
    ///
    /// Usefull for compatibilty with pkg-config-rs up to 0.3.9 or to deal with
    /// `./configure` scripts.
    fn with_pkgconfig(&mut self) -> Result<&mut Command>;

    /// Propagate TARGET, TARGET_CC, TARGET_AR and TARGET_SYSROOT to a
    /// `./configure` script.
    fn with_toolchain(&mut self) -> Result<&mut Command>;
}

impl CommandExt for Command {
    fn arg_for_macos<S: AsRef<OsStr>>(&mut self, arg: S) -> Result<&mut Command> {
        if env::var("TARGET").map(|target| target.contains("-apple-darwin")).unwrap_or(false) {
            self.arg(arg.as_ref());
        }
        Ok(self)
    }

    fn configure_prefix<P: AsRef<Path>>(&mut self, prefix_dir: P) -> Result<&mut Command> {
        self.args(&["--prefix", path_to_str(&path_between(
            sysroot_path().unwrap_or(PathBuf::from("/")),
            prefix_dir))?]);
        Ok(self)
    }

    fn with_pkgconfig(&mut self) -> Result<&mut Command> {
        if is_cross_compiling()? {
            if let Ok(value) = target_env("PKG_CONFIG_PATH") {
                info!("Running command with PKG_CONFIG_PATH:{:?}", value);
                self.env("PKG_CONFIG_PATH", value);
            }
            if let Ok(value) = target_env("PKG_CONFIG_LIBDIR") {
                info!("Running command with PKG_CONFIG_LIBDIR:{:?}", value);
                self.env("PKG_CONFIG_LIBDIR", value);
            }
            if let Ok(value) = target_env("PKG_CONFIG_SYSROOT_DIR") {
                info!("Running command with PKG_CONFIG_SYSROOT_DIR:{:?}", value);
                self.env("PKG_CONFIG_SYSROOT_DIR", value);
            }
        }
        Ok(self)
    }

    fn with_toolchain(&mut self) -> Result<&mut Command> {
        if is_cross_compiling()? {
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
        }
        Ok(self)
    }
}


pub fn new_bindgen_with_cross_compilation_support() -> Result<bindgen::Builder> {
    Ok(bindgen::Builder::default()
        .clang_arg("--verbose")
        .detect_toolchain()?
        .include_gcc_system_headers()?
        .apple_patch()?)
}

pub trait BindGenBuilderExt {
    fn apple_patch(self) -> Result<bindgen::Builder>;

    fn detect_toolchain(self) -> Result<bindgen::Builder>;

    fn generate_default_binding(self) -> Result<()>;

    fn header_in_current_dir(self, header_file_name: &str) -> Result<bindgen::Builder>;

    fn include_gcc_system_headers(self) -> Result<bindgen::Builder>;
}

impl BindGenBuilderExt for bindgen::Builder {
    fn apple_patch(self) -> Result<bindgen::Builder> {
        if is_cross_compiling()? {
            let target = env::var("TARGET")?;
            if target.contains("apple") && target.contains("aarch64") {
                // The official Apple tools use "-arch arm64" instead of specifying
                // -target directly; -arch only works when the default target is
                // Darwin-based to put Clang into "Apple mode" as it were. But it does
                // sort of explain why arm64 works better than aarch64, which is the
                // preferred name everywhere else.
                return Ok(self
                    .clang_arg(format!("-arch"))
                    .clang_arg(format!("arm64")));
            }
        }
        Ok(self)
    }

    fn detect_toolchain(self) -> Result<bindgen::Builder> {
        if is_cross_compiling()? {
            let target = env::var("TARGET")?;
            Ok(self
                .clang_arg(format!("--sysroot={}", path_to_str(&sysroot_path()?)?))
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
        if is_cross_compiling()? {
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
