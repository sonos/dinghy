use std::{env, fs, path, process};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use cargo;

use errors::*;

use cargo::util::important_paths::find_root_manifest_for_wd;

pub fn setup_linker(device_target: &str) -> Result<()> {
    let cfg = cargo::util::config::Config::default()?;
    if let Some(linker) = cfg.get_string(&*format!("target.{}.linker", device_target))? {
        debug!("Config specifies linker {:?} in {}",
               linker.val,
               linker.definition);
        return Ok(());
    }
    let wd_path = find_root_manifest_for_wd(None, &env::current_dir()?)?;
    let root = wd_path.parent().ok_or("building at / ?")?;
    if let Some(linker) = guess_linker(device_target)? {
        let shim = create_shim(&root, device_target, &*linker)?;
        let var_name = format!("CARGO_TARGET_{}_LINKER",
                               device_target.replace("-", "_").to_uppercase());
        env::set_var(var_name, shim);
        return Ok(());
    }
    warn!("No linker set or guessed for target {}. See http://doc.crates.io/config.html .",
          device_target);
    Ok(())
}

#[cfg(not(target_os="windows"))]
fn create_shim<P: AsRef<path::Path>>(root: P,
                                     device_target: &str,
                                     shell: &str)
                                     -> Result<path::PathBuf> {
    let target_path = root.as_ref().join("target").join(device_target);
    fs::create_dir_all(&target_path)?;
    let shim = target_path.join("linker");
    if shim.exists() {
        return Ok(shim);
    }
    let mut linker_shim = fs::File::create(&shim)?;
    writeln!(linker_shim, "#!/bin/sh")?;
    linker_shim.write_all(shell.as_bytes())?;
    writeln!(linker_shim, "\n")?;
    fs::set_permissions(&shim, PermissionsExt::from_mode(0o777))?;
    Ok(shim)
}

#[cfg(target_os="windows")]
fn create_shim<P: AsRef<path::Path>>(root: P,
                                     device_target: &str,
                                     shell: &str)
                                     -> Result<path::PathBuf> {
    let target_path = root.as_ref().join("target").join(device_target);
    fs::create_dir_all(&target_path)?;
    let shim = target_path.join("linker.bat");
    let mut linker_shim = fs::File::create(&shim)?;
    linker_shim.write_all(shell.as_bytes())?;
    writeln!(linker_shim, "\n")?;
    Ok(shim)
}

#[cfg(not(target_os="windows"))]
fn guess_linker(device_target: &str) -> Result<Option<String>> {
    if device_target.ends_with("-apple-ios") {
        let xcrun = if device_target.starts_with("x86") {
            process::Command::new("xcrun").args(&["--sdk", "iphonesimulator", "--show-sdk-path"])
                .output()?
        } else {
            process::Command::new("xcrun").args(&["--sdk", "iphoneos", "--show-sdk-path"]).output()?
        };
        let sdk_path = String::from_utf8(xcrun.stdout)?;
        Ok(Some(format!(r#"cc -isysroot {} "$@""#, &*sdk_path.trim_right())))
    } else if device_target.contains("-linux-android") {
        if let Err(_) = env::var("ANDROID_NDK_HOME") {
            if let Ok(home) = env::var("HOME") {
                let mac_place = format!("{}/Library/Android/sdk/ndk-bundle", home);
                if fs::metadata(&mac_place)?.is_dir() {
                    env::set_var("ANDROID_NDK_HOME", &mac_place)
                }
            } else {
                warn!("Android target detected, but could not find (or guess) ANDROID_NDK_HOME. \
                       You may need to set it up.");
                return Ok(None);
            }
        }

        let (toolchain, gcc, arch) = match device_target {
            "armv7-linux-androideabi" => ("arm-linux-androideabi", "arm-linux-androideabi", "arch-arm"),
            "aarch64-linux-android" => (device_target, device_target, "arch-arm64"),
            "i686-linux-android" => ("x86", device_target, "arch-x86"),
            _ => (device_target, device_target, "arch-arm"),
        };

        let home = env::var("ANDROID_NDK_HOME")
                .map_err(|_| "environment variable ANDROID_NDK_HOME is required")?;
                
        let api = env::var("ANDROID_API")
                .unwrap_or(default_api_for_arch(arch)?.into());

        let prebuilt_dir = format!(r"{home}/toolchains/{toolchain}-4.9/prebuilt",
            home = home, toolchain = toolchain);

        let prebuilt = fs::read_dir(path::Path::new(&prebuilt_dir))?
            .next()
            .ok_or("No prebuilt toolchain in your android setup")??;

        Ok(Some(format!(r#"{prebuilt_dir}/{prebuilt:?}/bin/{gcc}-gcc \
            --sysroot {home}/platforms/{api}/{arch} \
            "$@" "#,
            prebuilt_dir = prebuilt_dir,
            prebuilt = prebuilt.file_name(),
            gcc = gcc,
            home = home,
            api = api,
            arch = arch)))
    } else {
        Ok(None)
    }
}

#[cfg(target_os="windows")]
fn guess_linker(device_target: &str) -> Result<Option<String>> {
    if device_target.contains("-linux-android") {
        let (toolchain, gcc, arch) = match device_target {
            "armv7-linux-androideabi" => ("arm-linux-androideabi", "arm-linux-androideabi", "arch-arm"),
            "aarch64-linux-android" => (device_target, device_target, "arch-arm64"),
            "i686-linux-android" => ("x86", device_target, "arch-x86"),
            _ => (device_target, device_target, "arch-arm"),
        };
        let home = env::var("ANDROID_NDK_HOME")
                .map_err(|_| "environment variable ANDROID_NDK_HOME is required")?;
        let api = env::var("ANDROID_API")
                .unwrap_or(default_api_for_arch(arch)?.into());

        let prebuilt_dir = format!(r"{home}\toolchains\{toolchain}-4.9\prebuilt",
            home = home, toolchain = toolchain);
        if !::std::path::Path::new(&prebuilt_dir).exists() {
            return Err(Error::from(format!("Could not find prebuilt android toolchain at:\n{:?}", prebuilt_dir)));
        }

        let mut toolchain_bin = prebuilt_dir.clone() + r"\windows-x86_64\bin";
        if !::std::path::Path::new(&toolchain_bin).exists() {
            toolchain_bin = prebuilt_dir + r"\windows\bin";
        }

        Ok(Some(format!(r"{toolchain_bin}\{gcc}-gcc --sysroot {home}\platforms\{api}\{arch} %* ",
            toolchain_bin = toolchain_bin,
            gcc = gcc,
            home = home,
            api = api,
            arch = arch)))
    } else {
        Ok(None)
    }
}

fn default_api_for_arch(android_arch: &str) -> Result<&'static str> {
    Ok(
        match android_arch {
        "arch-arm" => "android-18",
        "arch-arm64" => "android-21",
        "arch-mips" => "android-18",
        "arch-mips64" => "android-21",
        "arch-x86" => "android-18",
        "arch-x86_64" => "android-21",
        _ => return Err(Error::from(format!("Unknown android arch {}", android_arch)))
    })
}