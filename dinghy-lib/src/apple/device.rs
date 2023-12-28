use super::{xcode, AppleSimulatorType};
use crate::device::make_remote_app_with_name;
use crate::errors::*;
use crate::apple::AppleDevicePlatform;
use crate::project::Project;
use crate::utils::LogCommandExt;
use crate::utils::{get_current_verbosity, user_facing_log};
use crate::Build;
use crate::BuildBundle;
use crate::Device;
use crate::DeviceCompatibility;
use crate::Runnable;
use log::debug;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fs;
use std::path::Path;
use std::process;

#[derive(Clone, Debug)]
pub struct IosDevice {
    pub id: String,
    pub name: String,
    arch_cpu: &'static str,
    rustc_triple: String,
}

#[derive(Clone, Debug)]
pub struct AppleSimDevice {
    pub id: String,
    pub name: String,
    pub os: String,
    pub sim_type: AppleSimulatorType,
}

unsafe impl Send for IosDevice {}

impl IosDevice {
    pub fn new(json: &json::JsonValue) -> Result<IosDevice> {
        let device = &json["Device"];
        let id = device["DeviceIdentifier"]
            .as_str()
            .context("DeviceIdentifier expected to be a string")?
            .to_owned();
        let name = device["DeviceName"]
            .as_str()
            .context("DeviceName expected to be a string")?
            .to_owned();
        let arch_cpu = device["modelArch"]
            .as_str()
            .context("DeviceName expected to be a string")?;
        let cpu = match &*arch_cpu {
            "arm64" | "arm64e" => "aarch64",
            _ => "armv7",
        };
        Ok(IosDevice {
            name: name,
            id: id,
            arch_cpu: cpu.into(),
            rustc_triple: format!("{}-apple-ios", cpu),
        })
    }

    fn make_app(
        &self,
        project: &Project,
        build: &Build,
        runnable: &Runnable,
    ) -> Result<BuildBundle> {
        let signing = xcode::look_for_signature_settings(&self.id)?
            .pop()
            .ok_or_else(|| anyhow!("no signing identity found"))?;
        let app_id = signing
            .name
            .split(" ")
            .last()
            .ok_or_else(|| anyhow!("no app id ?"))?;

        let build_bundle = make_ios_app(project, build, runnable, &app_id)?;

        super::xcode::sign_app(&build_bundle, &signing)?;
        Ok(build_bundle)
    }

    fn install_app(
        &self,
        project: &Project,
        build: &Build,
        runnable: &Runnable,
    ) -> Result<BuildBundle> {
        user_facing_log(
            "Installing",
            &format!("{} to {}", build.runnable.id, self.id),
            0,
        );
        let build_bundle = self.make_app(project, build, runnable)?;
        let bundle = build_bundle.bundle_dir.to_string_lossy();
        let status = process::Command::new("ios-deploy")
            .args(&["-i", &self.id, "-b", &bundle, "-n"])
            .log_invocation(1)
            .output()
            .context("Failed to run ios-deploy")?
            .status;
        if !status.success() {
            bail!("Installation on device failed")
        }
        Ok(build_bundle)
    }

    fn run_remote(
        &self,
        app_path: &str,
        args: &[&str],
        envs: &[&str],
        debugger: bool,
    ) -> Result<()> {
        let mut command = process::Command::new("ios-deploy");
        command.args(&["-i", &self.id, "-b", &app_path, "-m"]);
        command.args(&["-a", &args.join(" ")]);
        command.args(&["-s", &envs.join(" ")]);
        command.arg(if debugger { "-d" } else { "-I" });
        command.stderr(process::Stdio::inherit());
        command.stdout(process::Stdio::inherit());
        let status = command
            .log_invocation(1)
            .output()
            .context("Failed to run ios-deploy")?
            .status;
        if !status.success() {
            bail!("Run on device failed")
        }
        Ok(())
    }
}

impl Device for IosDevice {
    fn clean_app(&self, _build_bundle: &BuildBundle) -> Result<()> {
        unimplemented!()
    }

    fn debug_app(
        &self,
        project: &Project,
        build: &Build,
        args: &[&str],
        envs: &[&str],
    ) -> Result<BuildBundle> {
        let build_bundle = self.install_app(project, build, &build.runnable)?;
        let bundle = build_bundle.bundle_dir.to_string_lossy();
        if get_current_verbosity() < 1 {
            // we log the full command for verbosity > 1, just log a short message when the user
            // didn't ask for verbose output
            user_facing_log(
                "Debugging",
                &format!("{} on {}", build.runnable.id, self.id),
                0,
            );
        }
        self.run_remote(&bundle, args, envs, true)?;
        Ok(build_bundle)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn run_app(
        &self,
        project: &Project,
        build: &Build,
        args: &[&str],
        envs: &[&str],
    ) -> Result<BuildBundle> {
        let build_bundle = self.install_app(project, build, &build.runnable)?;
        let bundle = build_bundle.bundle_dir.to_string_lossy();
        if get_current_verbosity() < 1 {
            // we log the full command for verbosity > 1, just log a short message when the user
            // didn't ask for verbose output
            user_facing_log(
                "Running",
                &format!("{} on {}", build.runnable.id, self.id),
                0,
            );
        }
        self.run_remote(&bundle, args, envs, false)?;
        Ok(build_bundle)
    }
}

impl AppleSimDevice {
    fn install_app(
        &self,
        project: &Project,
        build: &Build,
        runnable: &Runnable,
    ) -> Result<BuildBundle> {
        user_facing_log(
            "Installing",
            &format!("{} to {}", build.runnable.id, self.id),
            0,
        );
        let build_bundle = self.make_app(project, build, runnable)?;
        let _ = process::Command::new("xcrun")
            .args(&["simctl", "uninstall", &self.id, "Dinghy"])
            .log_invocation(2)
            .status()?;
        let stat = process::Command::new("xcrun")
            .args(&[
                "simctl",
                "install",
                &self.id,
                build_bundle
                    .bundle_dir
                    .to_str()
                    .ok_or_else(|| anyhow!("conversion to string"))?,
            ])
            .log_invocation(1)
            .status()?;
        if stat.success() {
            Ok(build_bundle)
        } else {
            bail!(
                "Failed to install {} for {}",
                runnable.exe.display(),
                self.id
            )
        }
    }

    fn make_app(&self, project: &Project, build: &Build, runnable: &Runnable) -> Result<BuildBundle> {
        match self.sim_type {
            AppleSimulatorType::Ios => make_ios_app(project, build, runnable, "Dinghy"),
            AppleSimulatorType::Watchos => make_watchos_app(project, build, runnable, "Dinghy"),
            AppleSimulatorType::Tvos => make_tvos_app(project, build, runnable, "Dinghy"),
        }
    }
}

impl Device for AppleSimDevice {
    fn clean_app(&self, _build_bundle: &BuildBundle) -> Result<()> {
        unimplemented!()
    }

    fn debug_app(
        &self,
        project: &Project,
        build: &Build,
        args: &[&str],
        envs: &[&str],
    ) -> Result<BuildBundle> {
        let runnable = &build.runnable;
        let build_bundle = self.install_app(project, build, runnable)?;
        let install_path = String::from_utf8(
            process::Command::new("xcrun")
                .args(&["simctl", "get_app_container", &self.id, "Dinghy"])
                .log_invocation(2)
                .output()?
                .stdout,
        )?;
        if get_current_verbosity() < 1 {
            // we log the full command for verbosity > 1, just log a short message when the user
            // didn't ask for verbose output
            user_facing_log(
                "Debugging",
                &format!("{} on {}", build.runnable.id, self.id),
                0,
            );
        }
        launch_lldb_simulator(&self, &install_path, args, envs, true)?;
        Ok(build_bundle)
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn run_app(
        &self,
        project: &Project,
        build: &Build,
        args: &[&str],
        envs: &[&str],
    ) -> Result<BuildBundle> {
        let build_bundle = self.install_app(&project, &build, &build.runnable)?;
        if get_current_verbosity() < 1 {
            // we log the full command for verbosity > 1, just log a short message when the user
            // didn't ask for verbose output
            user_facing_log(
                "Running",
                &format!("{} on {}", build.runnable.id, self.id),
                0,
            );
        }
        launch_app(&self, args, envs)?;
        Ok(build_bundle)
    }
}

impl Display for IosDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Ok(fmt.write_str(
            format!(
                "IosDevice {{ \"id\": \"{}\", \"name\": {}, \"arch_cpu\": {} }}",
                self.id, self.name, self.arch_cpu
            )
            .as_str(),
        )?)
    }
}

impl Display for AppleSimDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Ok(fmt.write_str(
            format!(
                "AppleSimDevice {{ \"id\": \"{}\", \"name\": {}, \"os\": {} }}",
                self.id, self.name, self.os
            )
            .as_str(),
        )?)
    }
}

impl DeviceCompatibility for IosDevice {
    fn is_compatible_with_simulator_platform(&self, platform: &AppleDevicePlatform) -> bool {
        if platform.sim.is_some() {
            return false;
        }

        if platform.toolchain.rustc_triple == self.rustc_triple.as_str() {
            return true;
        }
        return false;
    }
}

impl DeviceCompatibility for AppleSimDevice {
    fn is_compatible_with_simulator_platform(&self, platform: &AppleDevicePlatform) -> bool {
        if let Some(sim) = &platform.sim {
            self.sim_type == *sim
        } else {
            false
        }
    }
}

fn make_ios_app(
    project: &Project,
    build: &Build,
    runnable: &Runnable,
    app_id: &str,
) -> Result<BuildBundle> {
    use crate::project;
    let build_bundle = make_remote_app_with_name(project, build, Some("Dinghy.app"))?;
    project::rec_copy(&runnable.exe, build_bundle.bundle_dir.join("Dinghy"), false)?;
    let magic = process::Command::new("file")
        .arg(
            runnable
                .exe
                .to_str()
                .ok_or_else(|| anyhow!("path conversion to string: {:?}", runnable.exe))?,
        )
        .log_invocation(3)
        .output()?;
    let magic = String::from_utf8(magic.stdout)?;
    let target = magic
        .split(" ")
        .last()
        .ok_or_else(|| anyhow!("empty magic"))?;
    xcode::add_plist_to_ios_app(&build_bundle, target, app_id)?;
    Ok(build_bundle)
}

fn make_watchos_app(
    project: &Project,
    build: &Build,
    runnable: &Runnable,
    app_id: &str,
) -> Result<BuildBundle> {
    use crate::project;
    let build_bundle = make_remote_app_with_name(project, build, Some("Dinghy.app"))?;
    project::rec_copy(&runnable.exe, build_bundle.bundle_dir.join("Dinghy"), false)?;
    let magic = process::Command::new("file")
        .arg(
            runnable
                .exe
                .to_str()
                .ok_or_else(|| anyhow!("path conversion to string: {:?}", runnable.exe))?,
        )
        .log_invocation(3)
        .output()?;
    let magic = String::from_utf8(magic.stdout)?;
    let target = magic
        .split(" ")
        .last()
        .ok_or_else(|| anyhow!("empty magic"))?;
    xcode::add_plist_to_watchos_app(&build_bundle, target, app_id)?;
    Ok(build_bundle)
}

fn make_tvos_app(
    project: &Project,
    build: &Build,
    runnable: &Runnable,
    app_id: &str,
) -> Result<BuildBundle> {
    use crate::project;
    let build_bundle = make_remote_app_with_name(project, build, Some("Dinghy.app"))?;
    project::rec_copy(&runnable.exe, build_bundle.bundle_dir.join("Dinghy"), false)?;
    let magic = process::Command::new("file")
        .arg(
            runnable
                .exe
                .to_str()
                .ok_or_else(|| anyhow!("path conversion to string: {:?}", runnable.exe))?,
        )
        .log_invocation(3)
        .output()?;
    let magic = String::from_utf8(magic.stdout)?;
    let target = magic
        .split(" ")
        .last()
        .ok_or_else(|| anyhow!("empty magic"))?;
    xcode::add_plist_to_tvos_app(&build_bundle, target, app_id)?;
    Ok(build_bundle)
}

fn launch_app(dev: &AppleSimDevice, app_args: &[&str], _envs: &[&str]) -> Result<()> {
    use std::io::Write;
    let dir = ::tempdir::TempDir::new("mobiledevice-rs-lldb")?;
    let tmppath = dir.path();
    let mut install_path = String::from_utf8(
        process::Command::new("xcrun")
            .args(&["simctl", "get_app_container", &dev.id, "Dinghy"])
            .log_invocation(2)
            .output()?
            .stdout,
    )?;
    install_path.pop();
    let stdout = Path::new(&install_path)
        .join("stdout")
        .to_string_lossy()
        .into_owned();
    let stdout_param = &format!("--stdout={}", stdout);
    let mut xcrun_args: Vec<&str> = vec!["simctl", "launch", "-w", stdout_param, &dev.id, "Dinghy"];
    xcrun_args.extend(app_args);
    debug!("Launching app via xcrun using args: {:?}", xcrun_args);
    let launch_output = process::Command::new("xcrun")
        .args(&xcrun_args)
        .log_invocation(1)
        .output()?;
    let launch_output = String::from_utf8_lossy(&launch_output.stdout);
    debug!("xcrun simctl launch output: {:?}", launch_output);

    // Output from the launch command should be "Dinghy: $PID" which is after the 8th character.
    let dinghy_pid = launch_output.split_at(8).1;

    // Attaching to the processes needs to be done in a script, not a commandline parameter or
    // lldb will say "no simulators found".
    let lldb_script_filename = tmppath.join("lldb-script");
    let mut script = fs::File::create(&lldb_script_filename)?;
    write!(script, "attach {}\n", dinghy_pid)?;
    write!(script, "continue\n")?;
    write!(script, "quit\n")?;
    let output = process::Command::new("lldb")
        .arg("")
        .arg("-s")
        .arg(lldb_script_filename)
        .output()?;
    let test_contents = std::fs::read_to_string(stdout)?;
    println!("{}", test_contents);

    let output: String = String::from_utf8_lossy(&output.stdout).to_string();
    debug!("lldb script: \n{}", output);
    // The stdout from lldb is something like:
    //
    // (lldb) attach 34163
    // Process 34163 stopped
    // * thread #1, stop reason = signal SIGSTOP
    //     frame #0: 0x00000001019cd000 dyld`_dyld_start
    // dyld`_dyld_start:
    // ->  0x1019cd000 <+0>: popq   %rdi
    //     0x1019cd001 <+1>: pushq  $0x0
    //     0x1019cd003 <+3>: movq   %rsp, %rbp
    //     0x1019cd006 <+6>: andq   $-0x10, %rsp
    // Target 0: (Dinghy) stopped.
    // Executable module set to .....
    // Architecture set to: x86_64h-apple-ios-.
    // (lldb) continue
    // Process 34163 resuming
    // Process 34163 exited with status = 101 (0x00000065)
    // (lldb) quit
    //
    // We need the "exit with status" line which is the 3rd from the last
    let exit_status_line = output
        .lines()
        .rev()
        .find(|line| line.contains("exited with status"));
    if let Some(exit_status_line) = exit_status_line {
        let words: Vec<&str> = exit_status_line.split_whitespace().rev().collect();
        if let Some(exit_status) = words.get(1) {
            let exit_status = exit_status.parse::<u32>()?;
            if exit_status == 0 {
                Ok(())
            } else {
                bail!("Test failure, exit code: {}", exit_status)
            }
        } else {
            panic!(
                "Failed to parse lldb exit line for an exit status. {:?}",
                words
            );
        }
    } else {
        panic!("Failed to get the exit status line from lldb: {}", output);
    }
}

fn launch_lldb_simulator(
    dev: &AppleSimDevice,
    installed: &str,
    args: &[&str],
    envs: &[&str],
    debugger: bool,
) -> Result<()> {
    use std::io::Write;
    use std::process::Command;
    let dir = ::tempdir::TempDir::new("mobiledevice-rs-lldb")?;
    let tmppath = dir.path();
    let lldb_script_filename = tmppath.join("lldb-script");
    {
        let python_lldb_support = tmppath.join("helpers.py");
        let helper_py = include_str!("helpers.py");
        let helper_py = helper_py.replace("ENV_VAR_PLACEHOLDER", &envs.join("\", \""));
        fs::File::create(&python_lldb_support)?.write_fmt(format_args!("{}", &helper_py))?;
        let mut script = fs::File::create(&lldb_script_filename)?;
        writeln!(script, "platform select ios-simulator")?;
        writeln!(script, "target create {}", installed)?;
        writeln!(script, "script pass")?;
        writeln!(script, "command script import {:?}", python_lldb_support)?;
        writeln!(
            script,
            "command script add -s synchronous -f helpers.start start"
        )?;
        writeln!(
            script,
            "command script add -f helpers.connect_command connect"
        )?;
        writeln!(script, "connect connect://{}", dev.id)?;
        if !debugger {
            writeln!(script, "start {}", args.join(" "))?;
            writeln!(script, "quit")?;
        }
    }

    let stat = Command::new("xcrun")
        .arg("lldb")
        .arg("-Q")
        .arg("-s")
        .arg(lldb_script_filename)
        .log_invocation(1)
        .status()?;
    if stat.success() {
        Ok(())
    } else {
        bail!("LLDB returned error code {:?}", stat.code())
    }
}
