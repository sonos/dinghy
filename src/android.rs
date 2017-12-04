use std::{env, fs, path};
use std::process::{Command, Stdio};

use errors::*;
use {Device, PlatformManager, Platform};

#[derive(Debug, Clone)]
pub struct AndroidDevice {
    adb: String,
    id: String,
    supported_targets: Vec<&'static str>,
}

impl AndroidDevice {
    fn from_id(adb: String, id: &str) -> Result<AndroidDevice> {
        let getprop_output = Command::new(&adb)
            .args(&["-s", id, "shell", "getprop", "ro.product.cpu.abilist"])
            .output()?;
        let abilist = String::from_utf8(getprop_output.stdout)?;
        let supported_targets = abilist
            .trim()
            .split(",")
            .filter_map(|abi| {
                Some(match abi {
                    "arm64-v8a" => "aarch64-linux-android",
                    "armeabi-v7a" => "armv7-linux-androideabi",
                    "armeabi" => "arm-linux-androideabi",
                    "x86" => "i686-linux-android",
                    _ => return None,
                })
            })
            .collect::<Vec<_>>();

        let device = AndroidDevice {
            adb: adb,
            id: id.into(),
            supported_targets: supported_targets,
        };
        debug!("device: {:?}", device);
        Ok(device)
    }
}

impl Device for AndroidDevice {
    fn name(&self) -> &str {
        "android device"
    }
    fn id(&self) -> &str {
        &*self.id
    }
    fn target(&self) -> String {
        // Prefer arm-linux-androideabi if valid because it's Tier 1
        self.supported_targets
            .iter()
            .filter(|&s| s == &"arm-linux-androideabi")
            .next()
            .or_else(|| self.supported_targets.get(0))
            .unwrap_or(&"")
            .to_string()
    }
    fn can_run(&self, target: &str) -> bool {
        self.supported_targets.iter().any(|&t| t == target)
    }
    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
    }
    fn platform(&self, target: &str) -> Result<Box<Platform>> {
        toolchain(target)
    }
    fn make_app(&self, source: &path::Path, exe: &path::Path) -> Result<path::PathBuf> {
        use std::fs;
        let exe_file_name = exe.file_name()
            .expect("app should be a file in android mode");

        let bundle_path = exe.parent().ok_or("no parent")?.join("dinghy");
        let bundled_exe_path = bundle_path.join(exe_file_name);

        debug!("Removing previous bundle {:?}", bundle_path);
        let _ = fs::remove_dir_all(&bundle_path);

        debug!("Making bundle {:?} for {:?}", bundle_path, exe);
        fs::create_dir_all(&bundle_path)?;

        debug!("Copying exe to bundle");
        fs::copy(&exe, &bundled_exe_path)?;

        debug!("Copying src to bundle");
        ::rec_copy(source, &bundle_path, false)?;

        debug!("Copying test_data to bundle");
        ::copy_test_data(source, &bundle_path)?;

        Ok(bundled_exe_path.into())
    }
    fn install_app(&self, exe: &path::Path) -> Result<()> {
        let exe_name = exe.file_name()
            .and_then(|p| p.to_str())
            .expect("exe should be a file in android mode");
        let exe_parent = exe.parent()
            .and_then(|p| p.to_str())
            .expect("exe must have a parent");

        let target_dir = format!("/data/local/tmp/dinghy/{}", exe_name);
        let target_exec = format!("{}/{}", target_dir, exe_name);

        debug!("Clear existing files");
        let _stat = Command::new(&self.adb)
            .args(&["-s", &*self.id, "shell", "rm", "-rf", &*target_dir])
            .status()?;

        debug!("Push entire parent dir of exe");
        let stat = Command::new(&self.adb)
            .args(&["-s", &*self.id, "push", exe_parent, &*target_dir])
            .status()?;
        if !stat.success() {
            Err("failure in android install")?;
        }

        debug!("chmod target exe");
        let stat = Command::new(&self.adb)
            .args(&["-s", &*self.id, "shell", "chmod", "755", &*target_exec])
            .status()?;
        if !stat.success() {
            Err("failure in android install")?;
        }

        Ok(())
    }
    fn clean_app(&self, exe: &path::Path) -> Result<()> {
        let exe_name = exe.file_name()
            .and_then(|p| p.to_str())
            .expect("exe should be a file in android mode");

        let target_dir = format!("/data/local/tmp/dinghy/{}", exe_name);

        debug!("rm target exe");
        let stat = Command::new(&self.adb)
            .args(&["-s", &*self.id, "shell", "rm", "-rf", &*target_dir])
            .status()?;
        if !stat.success() {
            Err("failure in android clean")?;
        }

        Ok(())
    }
    fn run_app(&self, exe: &path::Path, args: &[&str], envs: &[&str]) -> Result<()> {
        let exe_name = exe.file_name()
            .and_then(|p| p.to_str())
            .expect("exe should be a file in android mode");

        let target_dir = format!("/data/local/tmp/dinghy/{}", exe_name);
        let target_exe = format!("{}/{}", target_dir, exe_name);

        let stat = Command::new(&self.adb)
            .arg("-s")
            .arg(&*self.id)
            .arg("shell")
            .arg(&*format!(
                "cd {:?}; DINGHY=1 {}",
                target_dir,
                envs.join(" ")
            ))
            .arg(&*target_exe)
            .args(args)
            .status()?;
        if !stat.success() {
            Err("failure in android run")?;
        }
        Ok(())
    }
    fn debug_app(&self, _app_path: &path::Path, _args: &[&str], _envs: &[&str]) -> Result<()> {
        unimplemented!()
    }
}

fn adb() -> Result<String> {
    fn try_out(command: &str) -> bool {
        match Command::new(command)
            .arg("--version")
            .stdout(Stdio::null())
            .status()
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }
    if try_out("fb_adb") {
        return Ok("fb-adb".into());
    }
    if try_out("adb") {
        return Ok("adb".into());
    }
    if let Ok(home) = env::var("HOME") {
        let mac_place = format!("{}/Library/Android/sdk/platform-tools/adb", home);
        if try_out(&mac_place) {
            return Ok(mac_place);
        }
    }
    Err("Neither fb-adb or adb could be found")?
}

pub struct AndroidManager {
    adb: String,
}

impl PlatformManager for AndroidManager {
    fn devices(&self) -> Result<Vec<Box<Device>>> {
        let result = Command::new(&self.adb).arg("devices").output()?;
        let mut devices = vec![];
        let device_regex = ::regex::Regex::new(r#"^(\S+)\tdevice\r?$"#)?;
        for line in String::from_utf8(result.stdout)?.split("\n").skip(1) {
            if let Some(caps) = device_regex.captures(line) {
                let d = AndroidDevice::from_id(self.adb.clone(), &caps[1])?;
                debug!("Discovered Android device {:?}", d);
                devices.push(Box::new(d) as Box<Device>);
            }
        }
        Ok(devices)
    }
}

impl AndroidManager {
    pub fn probe() -> Option<AndroidManager> {
        match adb() {
            Ok(adb) => {
                info!("Using {}", adb);
                Some(AndroidManager { adb })
            }
            Err(_) => {
                info!("adb not found in path, android disabled");
                None
            }
        }
    }
}

fn toolchain_path() -> Result<path::PathBuf> {
    let user_home = env::home_dir().ok_or("no home dir found'")?;
    Ok(user_home.join(".dinghy").join("android-toolchains"))
}

fn toolchain(target: &str) -> Result<Box<Platform>> {
    if toolchain_path()?.exists() {
        for f in toolchain_path()?.read_dir()? {
            let f = f?;
            if f.file_name().to_string_lossy().starts_with(target) {
                return Ok(::regular_platform::RegularPlatform::new(f.path())?);
            }
        }
    }
    AndroidNdk::for_target(target)
}

#[derive(Debug)]
pub struct AndroidNdk {
    toolchain: String,
    gcc_prefix: String,
    arch: String,
    home: String,
    api: String,
    prebuilt_dir: path::PathBuf,
}

impl Platform for AndroidNdk {
    fn cc_command(&self, _target: &str) -> Result<String> {
        let gcc = self.prebuilt_dir
            .join("bin")
            .join(format!("{}-gcc", self.gcc_prefix));
        Ok(format!("{:?} {}", gcc, ::shim::GLOB_ARGS))
    }

    fn linker_command(&self, _target: &str) -> Result<String> {
        let gcc = self.prebuilt_dir
            .join("bin")
            .join(format!("{}-gcc", self.gcc_prefix));
        Ok(format!("{:?} {}", gcc, ::shim::GLOB_ARGS))
    }
}

impl AndroidNdk {
    fn for_target(device_target: &str) -> Result<Box<Platform>> {
        if let Err(_) = env::var("ANDROID_NDK_HOME") {
            if let Some(home) = env::home_dir() {
                let mac_place = home.join("Library/Android/sdk/ndk-bundle");
                if fs::metadata(&mac_place)?.is_dir() {
                    env::set_var("ANDROID_NDK_HOME", &mac_place)
                }
            } else {
                Err(
                    "Android target detected, but could not find (or guess) ANDROID_NDK_HOME or a toolchain. \
                     You need to set it up.",
                )?
            }
        }

        let (toolchain, gcc, arch) = Self::ndk_details(device_target)?;

        let home = env::var("ANDROID_NDK_HOME")
            .map_err(|_| "environment variable ANDROID_NDK_HOME is required")?;

        let api = env::var("ANDROID_API").unwrap_or(Self::default_api_for_arch(arch)?.into());
        let wanted_tc = toolchain_path()?.join(format!("{}_{}", toolchain, api));

        warn!(
            "Using ndk as a toolchain for: {}. This only works for pure rust builds.",
            device_target
        );
        warn!("If your build has non-rust dependencies (like C or C++ libraries), consider building a standalone toolchain.");
        warn!("For instance:");
        warn!(
            "  python {:?} --arch {} --api {} --install-dir {:?}",
            path::Path::new(&home)
                .join("build")
                .join("tools")
                .join("make_standalone_toolchain.py"),
            arch.split("-").last().unwrap(),
            api.split("-").last().unwrap(),
            wanted_tc
        );

        let prebuilt_dir = path::Path::new(&home)
            .join("toolchains")
            .join(format!("{}-4.9", toolchain))
            .join("prebuilt");

        let prebuilt_dir = prebuilt_dir
            .read_dir()?
            .next()
            .ok_or("No prebuilt toolchain in your android setup")??;

        Ok(Box::new(AndroidNdk {
            toolchain: toolchain.into(),
            gcc_prefix: gcc.into(),
            arch: arch.into(),
            home: home.into(),
            api: api.into(),
            prebuilt_dir: prebuilt_dir.path().into(),
        }))
    }

    fn ndk_details(rust_target: &str) -> Result<(&str, &str, &str)> {
        Ok(match rust_target {
            "armv7-linux-androideabi" => {
                ("arm-linux-androideabi", "arm-linux-androideabi", "arch-arm")
            }
            "aarch64-linux-android" => (rust_target, rust_target, "arch-arm64"),
            "i686-linux-android" => ("x86", rust_target, "arch-x86"),
            _ => (rust_target, rust_target, "arch-arm"),
        })
    }

    fn default_api_for_arch(android_arch: &str) -> Result<&'static str> {
        Ok(match android_arch {
            "arch-arm" => "android-18",
            "arch-arm64" => "android-21",
            "arch-mips" => "android-18",
            "arch-mips64" => "android-21",
            "arch-x86" => "android-18",
            "arch-x86_64" => "android-21",
            _ => {
                return Err(Error::from(format!(
                    "Unknown android arch {}",
                    android_arch
                )))
            }
        })
    }
}

impl ::std::fmt::Display for AndroidNdk {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(f, "AndroidNdk")
    }
}
