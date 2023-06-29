use crate::device::make_remote_app;
use crate::errors::*;
use crate::platform::regular_platform::RegularPlatform;
use crate::project::Project;
use crate::utils::{get_current_verbosity, path_to_str, user_facing_log, LogCommandExt};
use crate::Build;
use crate::BuildBundle;
use crate::Device;
use crate::DeviceCompatibility;
use log::{debug, info, log_enabled};
use std::io::Write;
use std::{fmt, io, path, process};

static ANDROID_WORK_DIR: &str = "/data/local/tmp/dinghy";

#[derive(Clone)]
pub struct AndroidDevice {
    pub adb: path::PathBuf,
    pub id: String,
    pub supported_targets: Vec<&'static str>,
}

impl AndroidDevice {
    pub fn from_id(adb: path::PathBuf, id: &str) -> Result<AndroidDevice> {
        for prop in &[
            "ro.product.cpu.abilist",
            "ro.product.cpu.abi",
            "ro.product.cpu.abi2",
        ] {
            let getprop_output = process::Command::new(&adb)
                .args(&["-s", id, "shell", "getprop", prop])
                .log_invocation(3)
                .output()?;
            let abilist = String::from_utf8(getprop_output.stdout)?;
            debug!(
                "Android device {}, getprop {} returned {}",
                id,
                prop,
                abilist.trim()
            );
            if abilist.trim().len() > 0 {
                let supported_targets = abilist
                    .trim()
                    .split(",")
                    .filter_map(|abi| {
                        Some(match abi {
                            "arm64-v8a" => "aarch64-linux-android",
                            "armeabi-v7a" => "armv7-linux-androideabi",
                            "armeabi" => "arm-linux-androideabi",
                            "x86" => "i686-linux-android",
                            "x86_64" => "x86_64-linux-android",
                            _ => return None,
                        })
                    })
                    .collect::<Vec<_>>();

                return Ok(AndroidDevice {
                    adb,
                    id: id.into(),
                    supported_targets: supported_targets,
                });
            }
        }
        bail!("Could not match a platform to the device")
    }

    fn adb(&self) -> Result<process::Command> {
        let mut command = process::Command::new(&self.adb);
        command.arg("-s").arg(&self.id);
        Ok(command)
    }

    fn install_app(&self, project: &Project, build: &Build) -> Result<(BuildBundle, BuildBundle)> {
        info!("Install {} to {}", build.runnable.id, self.id);
        user_facing_log(
            "Installing",
            &format!("{} to {}", build.runnable.id, self.id),
            0,
        );
        if !self
            .adb()?
            .arg("shell")
            .arg("mkdir")
            .arg("-p")
            .arg(ANDROID_WORK_DIR)
            .log_invocation(2)
            .status()?
            .success()
        {
            bail!(
                "Failure to create dinghy work dir '{:?}' on target android device",
                ANDROID_WORK_DIR
            )
        }

        let build_bundle = make_remote_app(project, build)?;
        let remote_bundle = AndroidDevice::to_remote_bundle(&build_bundle)?;

        self.sync(
            &build_bundle.bundle_dir,
            &remote_bundle
                .bundle_dir
                .parent()
                .ok_or_else(|| anyhow!("Invalid path {}", remote_bundle.bundle_dir.display()))?,
        )?;
        self.sync(
            &build_bundle.lib_dir,
            &remote_bundle
                .lib_dir
                .parent()
                .ok_or_else(|| anyhow!("Invalid path {}", remote_bundle.lib_dir.display()))?,
        )?;

        debug!("Chmod target exe {}", remote_bundle.bundle_exe.display());
        if !self
            .adb()?
            .arg("shell")
            .arg("chmod")
            .arg("755")
            .arg(&remote_bundle.bundle_exe)
            .log_invocation(2)
            .status()?
            .success()
        {
            bail!("Failure in android install");
        }
        Ok((build_bundle, remote_bundle))
    }

    fn sync<FP: AsRef<path::Path>, TP: AsRef<path::Path>>(
        &self,
        from_path: FP,
        to_path: TP,
    ) -> Result<()> {
        // Seems overkill...
        // let _ = self.adb()?.arg("shell").arg("rm").arg("-rf").arg(to_path.as_ref()).status()?;
        // Need parent as adb

        let mut command = self.adb()?;
        command
            .arg("push")
            .arg("--sync")
            .arg(from_path.as_ref())
            .arg(to_path.as_ref());
        if !log_enabled!(::log::Level::Debug) {
            command.stdout(::std::process::Stdio::null());
            command.stderr(::std::process::Stdio::null());
        }
        debug!("Running {:?}", command);
        if !command.log_invocation(2).status()?.success() {
            bail!("Error syncing android directory ({:?})", command)
        } else {
            Ok(())
        }
    }

    fn to_remote_bundle(build_bundle: &BuildBundle) -> Result<BuildBundle> {
        build_bundle.replace_prefix_with(ANDROID_WORK_DIR)
    }
}

impl DeviceCompatibility for AndroidDevice {
    fn is_compatible_with_regular_platform(&self, platform: &RegularPlatform) -> bool {
        if platform.id.starts_with("auto-android") {
            let cpu = platform.id.split("-").nth(2).unwrap();
            self.supported_targets
                .iter()
                .any(|target| target.starts_with(cpu))
        } else {
            self.supported_targets
                .contains(&&*platform.toolchain.binutils_prefix)
        }
    }
}

impl Device for AndroidDevice {
    fn clean_app(&self, build_bundle: &BuildBundle) -> Result<()> {
        let remote_bundle = AndroidDevice::to_remote_bundle(build_bundle)?;
        debug!("Cleaup device");
        if !self
            .adb()?
            .arg("shell")
            .arg("rm")
            .arg("-rf")
            .arg(&remote_bundle.bundle_dir)
            .log_invocation(1)
            .status()?
            .success()
        {
            bail!("Failure in android clean")
        }
        if !self
            .adb()?
            .arg("shell")
            .arg("rm")
            .arg("-rf")
            .arg(&remote_bundle.lib_dir)
            .log_invocation(1)
            .status()?
            .success()
        {
            bail!("Failure in android clean")
        }
        Ok(())
    }

    fn debug_app(
        &self,
        _project: &Project,
        _build: &Build,
        _args: &[&str],
        _envs: &[&str],
    ) -> Result<BuildBundle> {
        unimplemented!()
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "android device"
    }

    fn run_app(
        &self,
        project: &Project,
        build: &Build,
        args: &[&str],
        envs: &[&str],
    ) -> Result<BuildBundle> {
        let args: Vec<String> = args
            .iter()
            .map(|&a| ::shell_escape::escape(a.into()).to_string())
            .collect();
        let (build_bundle, remote_bundle) = self.install_app(&project, &build)?;
        let command = format!(
                "cd '{}'; RUST_BACKTRACE=1 {} DINGHY=1 LD_LIBRARY_PATH=\"{}:$LD_LIBRARY_PATH\" {} {} ; echo FORWARD_RESULT_TO_DINGHY_BECAUSE_ADB_DOES_NOT=$?",
                path_to_str(&remote_bundle.bundle_dir)?,
                envs.join(" "),
                path_to_str(&remote_bundle.lib_dir)?,
                path_to_str(&remote_bundle.bundle_exe)?,
                args.join(" "));
        info!("Run {} on {}", build.runnable.id, self.id);

        if get_current_verbosity() < 1 {
            // we log the full command for verbosity > 1, just log a short message when the user
            // didn't ask for verbose output
            user_facing_log(
                "Running",
                &format!("{} on {}", build.runnable.id, self.id),
                0,
            );
        }

        if !self
            .adb()?
            .arg("shell")
            .arg(&command)
            .log_invocation(1)
            .output()
            .with_context(|| format!("Couldn't run {} using adb.", build.runnable.exe.display()))
            .and_then(|output| {
                if output.status.success() {
                    let _ = io::stdout().write(output.stdout.as_slice());
                    let _ = io::stderr().write(output.stderr.as_slice());
                    String::from_utf8(output.stdout).with_context(|| {
                        format!("Couldn't run {} using adb.", build.runnable.exe.display())
                    })
                } else {
                    bail!("Couldn't run {} using adb.", build.runnable.exe.display())
                }
            })
            .map(|output| output.lines().last().unwrap_or("").to_string())
            .map(|last_line| {
                last_line.contains("FORWARD_RESULT_TO_DINGHY_BECAUSE_ADB_DOES_NOT=0")
            })?
        {
            bail!("Failed")
        }

        Ok(build_bundle)
    }
}

impl fmt::Display for AndroidDevice {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Android/{}", self.id)
    }
}

impl fmt::Debug for AndroidDevice {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        Ok(fmt.write_str(
            format!(
                "Android {{ \"id\": \"{}\", \"supported_targets\": {:?} }}",
                self.id, self.supported_targets
            )
            .as_str(),
        )?)
    }
}
