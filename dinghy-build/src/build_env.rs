//! Target-aware environment manipulations.
//!
//! cc-rs and pkg-config-rs use a similar convention where some environment
//! variables (like CC, CFLAGS or PKG_CONFIG_PATH) can be tagged with the
//! current rustc target to distinguish a native build environment and one
//! or several cross-compilation ones.
//!
//! For instance, while compiling for Android arm, `cc-rs` looks first at
//! CC_arm-linux-androideabi, then CC_arm_linux_androideabi, the TARGET_CC
//! and finally CC.
//!
//! This crates implements some of the same logic and also helps generating
//! these variables names. It also notify all environment lookup "back" to
//! cargo using `cargo:rerun-if-env-changed` markup.

use anyhow::{Context, Result};
use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::PathBuf;

/// Append a value to a PATH-like (`:`-separated) environment variable.
pub fn append_path_to_env<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) {
    let mut formatted_value = OsString::new();
    if let Ok(initial_value) = env::var(key.as_ref()) {
        formatted_value.push(initial_value);
        formatted_value.push(":");
    }
    formatted_value.push(value);
    env::set_var(key.as_ref(), formatted_value);
}

/// Append a value to a PATH-like (`:`-separated) environment variable taking
/// target scoping rules into consideration.
pub fn append_path_to_target_env<K: AsRef<OsStr>, R: AsRef<str>, V: AsRef<OsStr>>(
    k: K,
    rustc_triple: Option<R>,
    v: V,
) {
    append_path_to_env(target_key_from_triple(k, rustc_triple), v.as_ref())
}

/// Build-context aware environment variable access.
///
/// If we are running in a build.rs context, register the var to cargo using
/// `cargo:rerun-if-env-changed`.
pub fn build_env(name: &str) -> Result<String> {
    let is_build_rs = env::var("CARGO_PKG_NAME").is_ok() && env::var("OUT_DIR").is_ok();

    if is_build_rs {
        println!("cargo:rerun-if-env-changed={}", name);
    }
    Ok(env::var(name)?)
}

/// Capitalize and replace `-` by `_`.
pub fn envify<S: AsRef<str>>(name: S) -> String {
    name.as_ref()
        .chars()
        .map(|c| c.to_ascii_uppercase())
        .map(|c| if c == '-' || c == '.' { '_' } else { c })
        .collect()
}

/// Set a bunch of environment variables.
pub fn set_all_env<K: AsRef<OsStr>, V: AsRef<OsStr>>(env: &[(K, V)]) {
    for env_var in env {
        set_env(env_var.0.as_ref(), env_var.1.as_ref())
    }
}

/// Set one environment variable.
pub fn set_env<K: AsRef<OsStr>, V: AsRef<OsStr>>(k: K, v: V) {
    log::debug!(
        "Setting environment variable {:?}={:?}",
        k.as_ref(),
        v.as_ref()
    );
    env::set_var(k, v);
}

/// Set one environment variable if not set yet.
pub fn set_env_ifndef<K: AsRef<OsStr>, V: AsRef<OsStr>>(k: K, v: V) {
    if let Ok(current_env_value) = env::var(k.as_ref()) {
        log::debug!(
            "Ignoring value {:?} as environment variable {:?} already defined with value {:?}",
            k.as_ref(),
            v.as_ref(),
            current_env_value
        );
    } else {
        log::debug!(
            "Setting environment variable {:?}={:?}",
            k.as_ref(),
            v.as_ref()
        );
        env::set_var(k, v);
    }
}

/// Set one environment variable with target-scoping rules.
pub fn set_target_env<K: AsRef<OsStr>, R: AsRef<str>, V: AsRef<OsStr>>(
    k: K,
    rustc_triple: Option<R>,
    v: V,
) {
    set_env(target_key_from_triple(k, rustc_triple), v);
}

/// Access a required TARGET_SYSROOT variable, suggesting to define it or use
/// Dinghy.
pub fn sysroot_path() -> Result<PathBuf> {
    env::var_os("TARGET_SYSROOT")
        .map(PathBuf::from)
        .context("You must either define a TARGET_SYSROOT or use Dinghy to build your project.")
}

/// Access `var_base` directly, or use targetting rules depending on the build
/// being native or cross.
pub fn target_env(var_base: &str) -> Result<String> {
    if let Ok(target) = env::var("TARGET") {
        let is_host = env::var("HOST")? == target;
        target_env_from_triple(var_base, target.as_str(), is_host)
    } else {
        build_env(var_base)
    }
}

/// Access `var_base` directly, using targetting rules.
pub fn target_env_from_triple(var_base: &str, triple: &str, is_host: bool) -> Result<String> {
    build_env(&format!("{}_{}", var_base, triple))
        .or_else(|_| build_env(&format!("{}_{}", var_base, triple.replace("-", "_"))))
        .or_else(|_| {
            build_env(&format!(
                "{}_{}",
                if is_host { "HOST" } else { "TARGET" },
                var_base
            ))
        })
        .or_else(|_| build_env(var_base))
}

fn target_key_from_triple<K: AsRef<OsStr>, R: AsRef<str>>(
    k: K,
    rustc_triple: Option<R>,
) -> OsString {
    let mut target_key = OsString::new();
    target_key.push(k);
    if let Some(rustc_triple) = rustc_triple {
        target_key.push("_");
        target_key.push(rustc_triple.as_ref().replace("-", "_"));
    }
    target_key
}
