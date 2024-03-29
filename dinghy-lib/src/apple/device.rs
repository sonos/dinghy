use super::{xcode, AppleSimulatorType};
use crate::apple::AppleDevicePlatform;
use crate::device::make_remote_app_with_name;
use crate::errors::*;
use crate::project::Project;
use crate::utils::LogCommandExt;
use crate::utils::{get_current_verbosity, user_facing_log};
use crate::Build;
use crate::BuildBundle;
use crate::Device;
use crate::DeviceCompatibility;
use crate::Runnable;
use colored::Colorize;
use itertools::Itertools;
use log::debug;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{self, Stdio};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct IosDevice {
    pub id: String,
    pub name: String,
    pub arch_cpu: &'static str,
    rustc_triple: String,
    pub os: String,
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
    pub fn new(name: String, id: String, arch_cpu: &str, os: String) -> Result<IosDevice> {
        let cpu = match &*arch_cpu {
            "arm64" | "arm64e" => "aarch64",
            _ => "armv7",
        };
        Ok(IosDevice {
            name,
            id,
            os,
            arch_cpu: cpu.into(),
            rustc_triple: format!("{}-apple-ios", cpu),
        })
    }

    fn is_pre_ios_17(&self) -> Result<bool> {
        Ok(semver::Version::parse(&self.os)?.major < 17)
    }

    fn is_locked(&self) -> Result<bool> {
        let result = process::Command::new("xcrun")
            .args(
                "devicectl device info lockState --quiet --json-output /dev/stdout --device"
                    .split_whitespace(),
            )
            .arg(&self.id)
            .log_invocation(1)
            .output()
            .context("Failed to run devicectl device info lockState")?;
        if !result.status.success() {
            bail!("Device lock query failed\n",)
        }
        Ok(
            json::parse(std::str::from_utf8(&result.stdout)?)?["result"]["passcodeRequired"]
                .as_bool()
                .unwrap(),
        )
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

        let build_bundle = make_apple_app(project, build, runnable, &app_id, None)?;

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
            &format!("{} to {} ({})", build.runnable.id, self.id, self.name),
            0,
        );
        let build_bundle = self.make_app(project, build, runnable)?;
        let bundle = build_bundle.bundle_dir.to_string_lossy();
        if self.is_pre_ios_17()? {
            self.install_app_with_ios_deploy(&bundle)?;
            return Ok(build_bundle);
        }

        // xcrun devicectl device install app --device 00008110-001XXXXXXXXXX ./xgen/Build/Products/Release-iphoneos/nilo.app
        let result = process::Command::new("xcrun")
            .args("devicectl device install app --device".split_whitespace())
            .arg(&self.id)
            .arg(&*bundle)
            .log_invocation(1)
            .status()
            .context("Failed to run devicectl device install app")?;
        if !result.success() {
            bail!("Installation on device failed\n",)
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
        if self.is_pre_ios_17()? {
            return self.run_remote_with_ios_deploy(app_path, args, envs, debugger);
        }
        let app_list = process::Command::new("pymobiledevice3")
            .args("apps list --no-color --udid".split_whitespace())
            .arg(&self.id)
            .output()?;
        let app_list = json::parse(std::str::from_utf8(&app_list.stdout)?).with_context(|| {
            format!(
                "Ran `pymobiledevice3 app list --no-color --udid {}`, could not parse expected JSON output.", self.id,
            )
        })?;
        let app = app_list
            .entries()
            .find(|e| e.0.ends_with("Dinghy"))
            .unwrap()
            .1;
        let remote_path = app["Path"].to_string();

        let tunnel = process::Command::new("sudo")
            .arg("-p")
            .arg(format!(
                "Please enter %p's password on %h to start a tunnel to '{}' (sudo):",
                self.name
            ))
            .args("pymobiledevice3 remote start-tunnel --script-mode --udid".split_whitespace())
            .arg(&self.id)
            .stderr(Stdio::inherit())
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped())
            .spawn()?;
        let mut rsd = String::new();
        BufReader::new(tunnel.stdout.unwrap()).read_line(&mut rsd)?;
        debug!("iOS RSD tunnel started: {rsd}");

        // start the debugserver
        let server = process::Command::new("pymobiledevice3")
            .args("developer debugserver start-server --rsd".split_whitespace())
            .args(rsd.trim().split_whitespace())
            .stderr(Stdio::inherit())
            .stdout(Stdio::piped())
            .spawn()?;
        let lldb_connection_string = BufReader::new(server.stdout.unwrap())
            .lines()
            .find(|l| l.as_ref().unwrap().contains("process connect connect://"))
            .unwrap()
            .unwrap();
        let connection_details = lldb_connection_string.split_whitespace().nth(3).unwrap();
        debug!("iOS debugserver started: {connection_details}");

        if self.is_locked()? {
            eprint!(
                "{}",
                format!("\n\n      Please unlock {}! ", &self.name).bright_yellow()
            );
            loop {
                std::thread::sleep(Duration::from_millis(300));
                if !self.is_locked()? {
                    eprintln!("{}", "   All good, yay!\n".bright_green());
                    break;
                }
            }
        }

        let tempdir = tempdir::TempDir::new("dinghy-lldb")?;
        let script_path = tempdir.path().join("run.lldb");
        // see https://stackoverflow.com/questions/77865860/lldb-hangs-when-trying-to-execute-command-with-o
        // for the terrible async thing
        std::fs::write(
            &script_path,
            format!(
                "
platform select remote-ios
target create {app_path}
script lldb.target.module[0].SetPlatformFileSpec(lldb.SBFileSpec('{remote_path}'))
script old_debug = lldb.debugger.GetAsync()
script lldb.debugger.SetAsync(True)
process connect {connection_details}
script lldb.debugger.SetAsync(old_debug)
run {}
exit
            ",
                args.iter()
                    .map(|&s| shell_escape::escape(s.into()))
                    .join(" ")
            ),
        )?;

        let lldb = process::Command::new("lldb")
            .arg("--batch")
            .arg("-s")
            .arg(script_path)
            .stderr(Stdio::inherit())
            .stdout(Stdio::piped())
            .spawn()?;
        let mut lines = BufReader::new(lldb.stdout.unwrap()).lines();
        while !lines.next().unwrap()?.starts_with("(lldb) run") {}
        for line in lines {
            let line = line?;
            println!("{}", line);
            if line.contains("exited with status = ") {
                let rv = line.split_whitespace().nth(6).unwrap();
                println!("returns: {rv}");
                if rv == "0" {
                    return Ok(());
                } else {
                    bail!("Failed")
                }
            }
        }
        Ok(())
    }

    // LEGACY IOS-DEPLOY BASED WORKFLOW (iOS<17)
    fn install_app_with_ios_deploy(&self, bundle: &str) -> Result<()> {
        process::Command::new("ios-deploy")
            .args(&["-i", &self.id, "-b", &bundle, "-n"])
            .log_invocation(1)
            .output()
            .context("Failed to run ios-deploy")?
            .status;
        Ok(())
    }

    fn run_remote_with_ios_deploy(
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

    fn make_app(
        &self,
        project: &Project,
        build: &Build,
        runnable: &Runnable,
    ) -> Result<BuildBundle> {
        make_apple_app(project, build, runnable, "Dinghy", Some(&self.sim_type))
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
        write!(
            fmt,
            "{} ({} {} {})",
            self.name, self.id, self.arch_cpu, self.os
        )
    }
}

impl Display for AppleSimDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "{} ({} sim {})", self.name, self.id, self.os)
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

fn make_apple_app(
    project: &Project,
    build: &Build,
    runnable: &Runnable,
    app_id: &str,
    sim_type: Option<&AppleSimulatorType>,
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
    xcode::add_plist_to_app(&build_bundle, target, app_id, sim_type)?;
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
