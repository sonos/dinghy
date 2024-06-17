//! Helpers for build.rs scripts.
//!
//! This library is meant to be used in build.rs scripts context.
//!
//! It contains a set of standalone functions that encodes some of the
//! shared wisdom and conventions across build.rs scripts, cargo, dinghy,
//! cc-rs, pkg-config-rs, bindgen, and others. It also helps providing
//! cross-compilation arguments to autotools `./configure` scripts.

mod bindgen_macros;
pub mod build;
pub mod build_env;
pub mod utils;

use crate::build::is_cross_compiling;
use crate::build_env::sysroot_path;
use crate::build_env::target_env;
use crate::utils::path_between;
use crate::utils::path_to_str;
use std::env;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

#[doc(hidden)]
pub use anyhow::{Context, Result};

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
        if env::var("TARGET")
            .map(|target| target.contains("-apple-darwin"))
            .unwrap_or(false)
        {
            self.arg(arg.as_ref());
        }
        Ok(self)
    }

    fn configure_prefix<P: AsRef<Path>>(&mut self, prefix_dir: P) -> Result<&mut Command> {
        self.args(&[
            "--prefix",
            path_to_str(&path_between(
                sysroot_path().unwrap_or(PathBuf::from("/")),
                prefix_dir,
            ))?,
        ]);
        Ok(self)
    }

    fn with_pkgconfig(&mut self) -> Result<&mut Command> {
        if is_cross_compiling()? {
            if let Ok(value) = target_env("PKG_CONFIG_PATH") {
                log::info!("Running command with PKG_CONFIG_PATH:{:?}", value);
                self.env("PKG_CONFIG_PATH", value);
            }
            if let Ok(value) = target_env("PKG_CONFIG_LIBDIR") {
                log::info!("Running command with PKG_CONFIG_LIBDIR:{:?}", value);
                self.env("PKG_CONFIG_LIBDIR", value);
            }
            if let Ok(value) = target_env("PKG_CONFIG_SYSROOT_DIR") {
                log::info!("Running command with PKG_CONFIG_SYSROOT_DIR:{:?}", value);
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
