use std::{env, fs, path};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;

use cargo;

use errors::*;

use cargo::util::important_paths::find_root_manifest_for_wd;

pub fn create_shim<P: AsRef<path::Path>>(root: P, device_target: &str) -> Result<()> {
    let target_path = root.as_ref().join("target").join(device_target);
    fs::create_dir_all(&target_path)?;
    let shim = target_path.join("linker");
    if shim.exists() {
        return Ok(())
    }
    let mut linker_shim = fs::File::create(&shim)?;
    writeln!(linker_shim, "#!/bin/sh")?;
    writeln!(linker_shim,
             "cc -isysroot \
              /Applications/Xcode.app/Contents/Developer/Platforms/iPhoneOS10.\
              platform/Developer/SDKs/iPhoneOS10.0.sdk \"$@\"")?;
    fs::set_permissions(shim, PermissionsExt::from_mode(0o777))?;
    Ok(())
}

fn ensure_shim(device_target: &str) -> Result<()> {
    let wd_path = find_root_manifest_for_wd(None, &env::current_dir()?)?;
    let root = wd_path.parent().ok_or("building at / ?")?;
    let target_path = root.join("target").join(device_target);
    create_shim(&root, &device_target)?;
    env::set_var("CARGO_TARGET_ARMV7_APPLE_IOS_LINKER", target_path.join("linker"));
    Ok(())
}

pub fn compile_tests(device_target: &str) -> Result<Vec<(String, path::PathBuf)>> {
    ensure_shim(device_target)?;
    let wd_path = find_root_manifest_for_wd(None, &env::current_dir()?)?;
    let cfg = cargo::util::config::Config::default()?;
    cfg.configure(0, None, &None, false, false)?;
    let wd = cargo::core::Workspace::new(&wd_path, &cfg)?;
    let options = cargo::ops::CompileOptions {
        config: &cfg,
        jobs: None,
        target: Some(&device_target),
        features: &[],
        all_features: false,
        no_default_features: false,
        spec: &[],
        filter: cargo::ops::CompileFilter::new(false, &[], &[], &[], &[]),
        release: false,
        mode: cargo::ops::CompileMode::Test,
        message_format: cargo::ops::MessageFormat::Human,
        target_rustdoc_args: None,
        target_rustc_args: None,
    };
    let compilation = cargo::ops::compile(&wd, &options)?;
    Ok(compilation.tests.iter().map(|t| (t.1.clone(), t.2.clone())).collect::<Vec<_>>())
}

pub fn compile_benches(device_target: &str) -> Result<Vec<(String, path::PathBuf)>> {
    ensure_shim(device_target)?;
    let wd_path = find_root_manifest_for_wd(None, &env::current_dir()?)?;
    let cfg = cargo::util::config::Config::default()?;
    cfg.configure(0, None, &None, false, false)?;
    let wd = cargo::core::Workspace::new(&wd_path, &cfg)?;
    let options = cargo::ops::CompileOptions {
        config: &cfg,
        jobs: None,
        target: Some(&device_target),
        features: &[],
        all_features: false,
        no_default_features: false,
        spec: &[],
        filter: cargo::ops::CompileFilter::new(false, &[], &[], &[], &[]),
        release: true,
        mode: cargo::ops::CompileMode::Bench,
        message_format: cargo::ops::MessageFormat::Human,
        target_rustdoc_args: None,
        target_rustc_args: None,
    };
    let compilation = cargo::ops::compile(&wd, &options)?;
    Ok(compilation.tests.iter().map(|t| (t.1.clone(), t.2.clone())).collect::<Vec<_>>())
}

pub fn compile_bin(device_target: &str) -> Result<Vec<path::PathBuf>> {
    ensure_shim(device_target)?;
    let wd_path = find_root_manifest_for_wd(None, &env::current_dir()?)?;
    let cfg = cargo::util::config::Config::default()?;
    cfg.configure(0, None, &None, false, false)?;
    let wd = cargo::core::Workspace::new(&wd_path, &cfg)?;
    let options = cargo::ops::CompileOptions {
        config: &cfg,
        jobs: None,
        target: Some(&device_target),
        features: &[],
        all_features: false,
        no_default_features: false,
        spec: &[],
        filter: cargo::ops::CompileFilter::new(false, &[], &[], &[], &[]),
        release: false,
        mode: cargo::ops::CompileMode::Build,
        message_format: cargo::ops::MessageFormat::Human,
        target_rustdoc_args: None,
        target_rustc_args: None,
    };
    let compilation = cargo::ops::compile(&wd, &options)?;
    Ok(compilation.binaries)
}

