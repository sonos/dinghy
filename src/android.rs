use std::path;
use std::process::Command;

use errors::*;
use ::{Device, PlatformManager};

#[derive(Debug,Clone)]
pub struct AndroidDevice {
    id: String,
    supported_targets: Vec<&'static str>,
}

impl AndroidDevice {
    fn from_id(id: &str) -> Result<AndroidDevice> {
        let getprop_output = Command::new(adb_bin_name()).args(&["-s", id,
            "shell", "getprop", "ro.product.cpu.abilist"])
            .output()?;
        let abilist = String::from_utf8(getprop_output.stdout)?;
        let supported_targets = abilist.trim().split(",").filter_map(|abi| {
            Some(
                match abi {
                "arm64-v8a" => "aarch64-linux-android",
                "armeabi-v7a" => "armv7-linux-androideabi",
                "armeabi" => "arm-linux-androideabi",
                "x86" => "i686-linux-android",
                _ => return None,
            })
        }).collect::<Vec<_>>();

        let device = AndroidDevice { id: id.into(), supported_targets: supported_targets };
        Ok(device)
    }
}

impl Device for AndroidDevice {
    fn name(&self) -> &str {
        "i'm a droid"
    }
    fn id(&self) -> &str {
        &*self.id
    }
    fn target(&self) -> String {
        // Prefer arm-linux-androideabi if valid because it's Tier 1
        self.supported_targets.iter()
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
    fn make_app(&self, source: &path::Path, exe: &path::Path) -> Result<path::PathBuf> {
        use std::fs;
        let exe_file_name = exe.file_name()
            .expect("app should be a file in android mode");

        let bundle_path = exe.parent().ok_or("no parent")?.join("dinghy");
        let bundled_exe_path = bundle_path.join(exe_file_name);

        debug!("Removing previous bundle {:?}", bundle_path);
        fs::remove_dir_all(&bundle_path)?;

        debug!("Making bundle {:?} for {:?}", bundle_path, exe);
        fs::create_dir_all(&bundle_path)?;

        debug!("Copying exe to bundle");
        fs::copy(&exe, &bundled_exe_path)?;

        debug!("Copying src to bundle");
        ::rec_copy(source, &bundle_path.join("src"))?;

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
        let _stat = Command::new(adb_bin_name()).args(&["-s", &*self.id,
            "shell", "rm", "-rf", &*target_dir])
            .status()?;

        debug!("Push entire parent dir of exe");
        let stat = Command::new("adb").args(&["-s", &*self.id,
            "push", exe_parent, &*target_dir])
            .status()?;
        if !stat.success() {
            Err("failure in android install")?;
        }

        debug!("chmod target exe");
        let stat = Command::new(adb_bin_name()).args(&["-s", &*self.id,
            "shell", "chmod", "755", &*target_exec])
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
        let stat = Command::new(adb_bin_name()).args(&["-s", &*self.id,
            "shell", "rm", "-rf", &*target_dir])
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

        let stat = Command::new(adb_bin_name())
            .arg("-s").arg(&*self.id)
            .arg("shell")
            .arg(&*format!("DINGHY=1 {}", envs.join(" ")))
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

fn adb_bin_name() -> &'static str {
    let status = Command::new("fb-adb")
            .arg("--version")
            .status();
            
    match status {
        Err(_) => "adb",
        Ok(_) => "fb-adb",
    }
}

pub struct AndroidManager {
}

impl PlatformManager for AndroidManager {
    fn devices(&self) -> Result<Vec<Box<Device>>> {
        let result = Command::new("adb").arg("devices").output()?;
        let mut devices = vec![];
        let device_regex = ::regex::Regex::new(r#"^(\S+)\tdevice\r?$"#)?;
        for line in String::from_utf8(result.stdout)?.split("\n").skip(1) {
            if let Some(caps) = device_regex.captures(line) {
                let d = AndroidDevice::from_id(&caps[1])?;
                debug!("Discovered Android device {:?}", d);
                devices.push(Box::new(d) as Box<Device>);
            }
        }
        Ok(devices)
    }
}

impl AndroidManager {
    pub fn probe() -> Option<AndroidManager> {
        match Command::new("adb").arg("devices").output() {
            Ok(_) => {
                info!("adb found in path, android enabled");
                Some(AndroidManager {})
            }
            Err(_) => {
                info!("adb not found in path, android disabled");
                None
            }
        }

    }
}
