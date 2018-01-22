use errors::*;
use device;
use platform::regular_platform::RegularPlatform;
use project::Project;
use std::env;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use utils::path_to_str;
use Build;
use Device;
use PlatformManager;
use DeviceCompatibility;
use Platform;
use BuildBundle;
use Runnable;

#[derive(Debug)]
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
            adb,
            id: id.into(),
            supported_targets: supported_targets,
        };
        debug!("device: {:?}", device);
        Ok(device)
    }

    fn to_remote_bundle(build_bundle: &BuildBundle) -> Result<(PathBuf, PathBuf)> {
        let remote_prefix = PathBuf::from("/data/local/tmp");
        let remote_dir = remote_prefix.join("dinghy").to_path_buf();
        let remote_exe = remote_dir.join(build_bundle.host_exe.file_name()
            .ok_or(format!("Invalid executable name '{}'", build_bundle.host_exe.display()))?)
            .to_path_buf();
        Ok((remote_dir, remote_exe))
    }
}

impl DeviceCompatibility for AndroidDevice {
    fn is_compatible_with_regular_platform(&self, platform: &RegularPlatform) -> bool {
        self.supported_targets.contains(&platform.toolchain.tc_triple.as_str())
    }
}

impl Device for AndroidDevice {
    fn name(&self) -> &str {
        "android device"
    }
    fn id(&self) -> &str {
        &self.id
    }
    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
    }
    fn clean_app(&self, build_bundle: &BuildBundle) -> Result<()> {
        let (remote_dir, _) = AndroidDevice::to_remote_bundle(build_bundle)?;
        debug!("rm target exe");
        let stat = Command::new(&self.adb)
            .arg("-s").arg(&self.id).arg("shell")
            .arg("rm").arg("-rf").arg(&remote_dir)
            .status()?;
        if !stat.success() {
            Err("Failure in android clean")?;
        }
        Ok(())
    }
    fn install_app(&self, project: &Project, build: &Build, runnable: &Runnable) -> Result<BuildBundle> {
        let build_bundle = device::make_app(project, build, runnable)?;
        let (remote_dir, remote_exe) = AndroidDevice::to_remote_bundle(&build_bundle)?;

        debug!("Clear existing files");
        let _stat = Command::new(&self.adb)
            .arg("-s").arg(&self.id).arg("shell").arg("rm").arg("-rf").arg(&remote_dir)
            .status()?;

        debug!("Push entire parent dir of exe");
        let stat = Command::new(&self.adb)
            .arg("-s").arg(&self.id).arg("push").arg(&build_bundle.host_dir).arg(&remote_dir)
            .status()?;
        if !stat.success() {
            Err("failure in android install")?;
        }

        debug!("chmod target exe");
        let stat = Command::new(&self.adb)
            .arg("-s").arg(&self.id).arg("shell").arg("chmod").arg("755").arg(&remote_exe)
            .status()?;
        if !stat.success() {
            Err("failure in android install")?;
        }
        Ok(build_bundle)
    }
    fn platform(&self) -> Result<Box<Platform>> {
        unimplemented!()
    }
    fn run_app(&self, build_bundle: &BuildBundle, args: &[&str], envs: &[&str]) -> Result<()> {
        let (remote_dir, remote_exe) = AndroidDevice::to_remote_bundle(&build_bundle)?;
        let status = Command::new(&self.adb)
            .arg("-s")
            .arg(&self.id)
            .arg("shell")
            .arg(&format!("cd {:?}; DINGHY=1 {}",
                          path_to_str(&remote_dir)?,
                          envs.join(" ")))
            .arg(&remote_exe)
            .args(args)
            .status()?;
        if !status.success() {
            Err("failure in android run")?;
        }
        Ok(())
    }
    fn debug_app(&self, _build_bundle: &BuildBundle, _args: &[&str], _envs: &[&str]) -> Result<()> {
        unimplemented!()
    }
}

impl Display for AndroidDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Ok(fmt.write_str(format!("Android {{ \"id\": \"{}\", \"supported_targets\": {:?} }}",
                                 self.id,
                                 self.supported_targets).as_str())?)
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
