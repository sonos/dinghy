use crate::device::make_remote_app;
use crate::project::Project;
use crate::utils::{get_current_verbosity, path_to_str, user_facing_log, LogCommandExt};
use crate::{platform::regular_platform::RegularPlatform, Device, DeviceCompatibility};
use crate::{Build, BuildBundle};
use anyhow::{anyhow, bail, Context, Result};
use log::{debug, info, log_enabled};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fmt, io};

static OHOS_WORK_DIR: &str = "/data/local/tmp/dinghy";

#[derive(Clone, Debug)]
pub struct OhosDevice {
    pub hdc: PathBuf,
    pub id: String,
    pub abi_list: Vec<String>,
}

impl OhosDevice {
    fn hdc(&self) -> Command {
        let mut command = Command::new(&self.hdc);
        command.arg("-t").arg(&self.id);
        command
    }

    fn to_remote_bundle(build_bundle: &BuildBundle) -> Result<BuildBundle> {
        build_bundle.replace_prefix_with(OHOS_WORK_DIR)
    }

    pub fn from_id(hdc: PathBuf, id: String) -> Result<OhosDevice> {
        // https://device.harmonyos.com/en/docs/apiref/doc-guides/faq-debugging-and-running-0000001122066466
        let abi_list = Command::new(&hdc)
            .args([
                "-t",
                &id,
                "shell",
                "param",
                "get",
                "const.product.cpu.abilist",
            ])
            .log_invocation(3)
            .output()?;
        let abi_list = String::from_utf8_lossy(&abi_list.stdout).into_owned();
        // Filter out hdc log(`[W][2024-02-15 16:34:34] FreeChannelContinue handle->data is nullptr`) and new line
        let abi_list = abi_list
            .lines()
            .map(|x| x.trim())
            .filter(|x| !x.is_empty())
            .filter(|x| !x.starts_with('['))
            .next()
            .context("Get ohos abi list failed.")?;
        debug!(
            "OpenHarmony device {}, get abi list returned `{}`",
            id, abi_list,
        );
        let abi_list = if abi_list == "default" {
            let output = Command::new(&hdc)
                .args(["-t", &id, "shell", "ls", "/system/"])
                .output()?
                .stdout;
            let output = String::from_utf8_lossy(&output);
            let lib64 = output
                .split(['\n', ' '])
                .filter(|x| !x.is_empty())
                .any(|x| x == "lib64");
            if lib64 {
                vec!["arm64-v8a".to_string()]
            } else {
                vec!["armeabi".to_string(), "armeabi-v7a".to_string()]
            }
        } else {
            abi_list
                .split(",")
                .map(|x| x.trim())
                .filter(|x| !x.is_empty())
                .map(|x| x.to_string())
                .collect()
        };
        Ok(OhosDevice { hdc, id, abi_list })
    }

    fn install_app(&self, project: &Project, build: &Build) -> Result<(BuildBundle, BuildBundle)> {
        info!("Install {} to {}", build.runnable.id, self.id);
        user_facing_log(
            "Installing",
            &format!("{} to {}", build.runnable.id, self.id),
            0,
        );
        if !self
            .hdc()
            .arg("shell")
            .arg("mkdir")
            .arg("-p")
            .arg(OHOS_WORK_DIR)
            .log_invocation(2)
            .status()?
            .success()
        {
            bail!(
                "Failure to create dinghy work dir '{:?}' on target ohos device",
                OHOS_WORK_DIR
            )
        }

        let build_bundle = make_remote_app(project, build)?;
        let remote_bundle = OhosDevice::to_remote_bundle(&build_bundle)?;

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
            .hdc()
            .arg("shell")
            .arg("chmod")
            .arg("755")
            .arg(&remote_bundle.bundle_exe)
            .log_invocation(2)
            .status()?
            .success()
        {
            bail!("Failure in ohos install");
        }
        Ok((build_bundle, remote_bundle))
    }

    fn sync<FP: AsRef<Path>, TP: AsRef<Path>>(&self, from_path: FP, to_path: TP) -> Result<()> {
        let mut command = self.hdc();
        command
            .arg("file")
            .arg("send")
            .arg("-sync")
            .arg(from_path.as_ref())
            .arg(to_path.as_ref());
        if !log_enabled!(::log::Level::Debug) {
            command.stdout(::std::process::Stdio::null());
            command.stderr(::std::process::Stdio::null());
        }
        debug!("Running {:?}", command);
        if !command.log_invocation(2).status()?.success() {
            bail!("Error syncing ohos directory ({:?})", command)
        } else {
            Ok(())
        }
    }
}

impl fmt::Display for OhosDevice {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "OpenHarmony/{}", self.id)
    }
}

impl DeviceCompatibility for OhosDevice {
    fn is_compatible_with_regular_platform(&self, platform: &RegularPlatform) -> bool {
        if let Some(abi) = platform.id.strip_prefix("auto-ohos-") {
            self.abi_list.iter().any(|x| x == abi)
        } else {
            false
        }
    }
}

impl Device for OhosDevice {
    fn clean_app(&self, build_bundle: &BuildBundle) -> Result<()> {
        let remote_bundle = OhosDevice::to_remote_bundle(build_bundle)?;
        debug!("Cleaup device");
        if !self
            .hdc()
            .arg("shell")
            .arg("rm")
            .arg("-rf")
            .arg(&remote_bundle.bundle_dir)
            .log_invocation(1)
            .status()?
            .success()
        {
            bail!("Failure in ohos clean")
        }
        if !self
            .hdc()
            .arg("shell")
            .arg("rm")
            .arg("-rf")
            .arg(&remote_bundle.lib_dir)
            .log_invocation(1)
            .status()?
            .success()
        {
            bail!("Failure in ohos clean")
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
        "OpenHarmony device"
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
                "cd '{}'; RUST_BACKTRACE=1 {} DINGHY=1 LD_LIBRARY_PATH=\"{}:$LD_LIBRARY_PATH\" {} {} ; echo FORWARD_RESULT_TO_DINGHY_BECAUSE_HDC_DOES_NOT=$?",
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
            .hdc()
            .arg("shell")
            .arg(&command)
            .log_invocation(1)
            .output()
            .with_context(|| format!("Couldn't run {} using hdc.", build.runnable.exe.display()))
            .and_then(|output| {
                if output.status.success() {
                    let _ = io::stdout().write(output.stdout.as_slice());
                    let _ = io::stderr().write(output.stderr.as_slice());
                    String::from_utf8(output.stdout).with_context(|| {
                        format!("Couldn't run {} using hdc.", build.runnable.exe.display())
                    })
                } else {
                    bail!("Couldn't run {} using hdc.", build.runnable.exe.display())
                }
            })
            .map(|output| {
                output
                    .lines()
                    // Filter out hdc logs
                    .filter(|x| !x.starts_with('['))
                    .last()
                    .unwrap_or("")
                    .to_string()
            })
            .map(|last_line| {
                last_line.contains("FORWARD_RESULT_TO_DINGHY_BECAUSE_HDC_DOES_NOT=0")
            })?
        {
            bail!("Failed")
        }

        Ok(build_bundle)
    }
}
