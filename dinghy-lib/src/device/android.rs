use errors::*;
use device::make_remote_app;
use platform::regular_platform::RegularPlatform;
use project::Project;
use std::env;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::io::stderr;
use std::io::stdout;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use utils::path_to_str;
use Build;
use BuildBundle;
use Device;
use DeviceCompatibility;
use PlatformManager;
use Runnable;


static ANDROID_WORK_DIR: &str = "/data/local/tmp/dinghy";

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
        debug!("device: {}", device);
        Ok(device)
    }

    fn adb(&self) -> Result<Command> {
        let mut command = Command::new(&self.adb);
        command.arg("-s").arg(&self.id);
        Ok(command)
    }

    fn install_app(&self, project: &Project, build: &Build, runnable: &Runnable) -> Result<(BuildBundle, BuildBundle)> {
        if !self.adb()?.arg("shell").arg("mkdir").arg("-p").arg(ANDROID_WORK_DIR).status()?.success() {
            Err(format!("Failure to create dinghy work dir '{}' on target android device", ANDROID_WORK_DIR))?;
        }

        let build_bundle = make_remote_app(project, build, runnable)?;
        let remote_bundle = AndroidDevice::to_remote_bundle(&build_bundle)?;

        self.sync(&build_bundle.bundle_dir, &remote_bundle.bundle_dir.parent()
            .ok_or(format!("Invalid path {}", remote_bundle.bundle_dir.display()))?)?;
        self.sync(&build_bundle.lib_dir, &remote_bundle.lib_dir.parent()
            .ok_or(format!("Invalid path {}", remote_bundle.lib_dir.display()))?)?;

        debug!("Chmod target exe {}", remote_bundle.bundle_exe.display());
        if !self.adb()?.arg("shell").arg("chmod").arg("755").arg(&remote_bundle.bundle_exe).status()?.success() {
            Err("Failure in android install")?;
        }
        Ok((build_bundle, remote_bundle))
    }

    fn sync<FP: AsRef<Path>, TP: AsRef<Path>>(&self, from_path: FP, to_path: TP) -> Result<()> {
        // Seems overkill...
        // let _ = self.adb()?.arg("shell").arg("rm").arg("-rf").arg(to_path.as_ref()).status()?;
        // Need parent as adb

        let mut command = self.adb()?;
        command.arg("push").arg("--sync").arg(from_path.as_ref()).arg(to_path.as_ref());
        debug!("Running {:?}", command);
        if !command.status()?.success() {
            bail!("Error syncing android directory ({:?})", command)
        } else {
            Ok(())
        }
    }

    fn to_remote_bundle(build_bundle: &BuildBundle) -> Result<BuildBundle> {
        build_bundle.replace_prefix_with(PathBuf::from(ANDROID_WORK_DIR))
    }
}

impl DeviceCompatibility for AndroidDevice {
    fn is_compatible_with_regular_platform(&self, platform: &RegularPlatform) -> bool {
        self.supported_targets.contains(&platform.toolchain.toolchain_triple.as_str())
    }
}

impl Device for AndroidDevice {
    fn clean_app(&self, build_bundle: &BuildBundle) -> Result<()> {
        let remote_bundle = AndroidDevice::to_remote_bundle(build_bundle)?;
        debug!("Cleaup device");
        if !self.adb()?.arg("shell").arg("rm").arg("-rf").arg(&remote_bundle.bundle_dir).status()?.success() {
            Err("Failure in android clean")?;
        }
        if !self.adb()?.arg("shell").arg("rm").arg("-rf").arg(&remote_bundle.lib_dir).status()?.success() {
            Err("Failure in android clean")?;
        }
        Ok(())
    }

    fn debug_app(&self, _project: &Project, _build: &Build, _args: &[&str], _envs: &[&str]) -> Result<BuildBundle> {
        unimplemented!()
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "android device"
    }

    fn run_app(&self, project: &Project, build: &Build, args: &[&str], envs: &[&str]) -> Result<Vec<BuildBundle>> {
        let mut build_bundles = vec![];
        for runnable in &build.runnables {
            let (build_bundle, remote_bundle) = self.install_app(&project, &build, &runnable)?;
            let command = format!(
                "cd '{}'; {} DINGHY=1 RUST_BACKTRACE=1 LD_LIBRARY_PATH=\"{}:$LD_LIBRARY_PATH\" {} {} ; echo FORWARD_RESULT_TO_DINGHY_BECAUSE_ADB_IS_CRAPPY=$?",
                path_to_str(&remote_bundle.bundle_dir)?,
                envs.join(" "),
                path_to_str(&remote_bundle.lib_dir)?,
                path_to_str(&remote_bundle.bundle_exe)?,
                args.join(" ")); // TODO Might cause issues with quoted args
            debug!("Running {}", command);

            if !self.adb()?
                .arg("shell")
                .arg(&command)
                .output()
                .chain_err(|| format!("Couldn't run {} using adb.", runnable.exe.display()))
                .and_then(|output| if output.status.success() {
                    let _ = stdout().write(output.stdout.as_slice());
                    let _ = stderr().write(output.stderr.as_slice());
                    String::from_utf8(output.stdout).chain_err(|| format!("Couldn't run {} using adb.", runnable.exe.display()))
                } else {
                    bail!("Couldn't run {} using adb.", runnable.exe.display())
                })
                .map(|output| output.lines().last().unwrap_or("").to_string())
                .map(|last_line| last_line.contains("FORWARD_RESULT_TO_DINGHY_BECAUSE_ADB_IS_CRAPPY=0"))? {
                Err("Test failed 🐛")?
            }

            build_bundles.push(build_bundle);
        }
        Ok(build_bundles)
    }

    fn start_remote_lldb(&self) -> Result<String> {
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
            .stderr(Stdio::null())
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
    if let Ok(android_home) = env::var("ANDROID_HOME") {
        let adb = format!("{}/platform-tools/adb", android_home);
        if try_out(&adb) { return Ok(adb); }
    }
    if let Ok(android_sdk) = env::var("ANDROID_SDK") {
        let adb = format!("{}/platform-tools/adb", android_sdk);
        if try_out(&adb) { return Ok(adb); }
    }
    if let Ok(android_sdk_home) = env::var("ANDROID_SDK_HOME") {
        let adb = format!("{}/platform-tools/adb", android_sdk_home);
        if try_out(&adb) { return Ok(adb); }
    }
    if let Ok(home) = env::var("HOME") {
        let mac_place = format!("{}/Library/Android/sdk/platform-tools/adb", home);
        if try_out(&mac_place) { return Ok(mac_place); }
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
                debug!("Discovered Android device {}", d);
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
