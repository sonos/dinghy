use super::xcode;
use crate::device::make_remote_app_with_name;
use crate::errors::*;
use crate::ios::IosPlatform;
use crate::project::Project;
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
pub struct IosSimDevice {
    pub id: String,
    pub name: String,
    pub os: String,
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
        let build_bundle = self.make_app(project, build, runnable)?;
        let bundle = build_bundle.bundle_dir.to_string_lossy();
        let status = process::Command::new("ios-deploy")
            .args(&["-i", &self.id, "-b", &bundle, "-n"])
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
        dbg!(&command);
        let status = command.output().context("Failed to run ios-deploy")?.status;
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
        self.run_remote(&bundle, args, envs, false)?;
        Ok(build_bundle)
    }

    fn start_remote_lldb(&self) -> Result<String> {
        unreachable!();
    }
}

impl IosSimDevice {
    fn install_app(
        &self,
        project: &Project,
        build: &Build,
        runnable: &Runnable,
    ) -> Result<BuildBundle> {
        let build_bundle = IosSimDevice::make_app(project, build, runnable)?;
        let _ = process::Command::new("xcrun")
            .args(&["simctl", "uninstall", &self.id, "Dinghy"])
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

    fn make_app(project: &Project, build: &Build, runnable: &Runnable) -> Result<BuildBundle> {
        make_ios_app(project, build, runnable, "Dinghy")
    }
}

impl Device for IosSimDevice {
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
                .output()?
                .stdout,
        )?;
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
        launch_app(&self, args, envs)?;
        Ok(build_bundle)
    }

    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
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

impl Display for IosSimDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Ok(fmt.write_str(
            format!(
                "IosSimDevice {{ \"id\": \"{}\", \"name\": {}, \"os\": {} }}",
                self.id, self.name, self.os
            )
            .as_str(),
        )?)
    }
}

impl DeviceCompatibility for IosDevice {
    fn is_compatible_with_ios_platform(&self, platform: &IosPlatform) -> bool {
        if platform.sim {
            return false;
        }

        if platform.toolchain.rustc_triple == self.rustc_triple.as_str() {
            return true;
        }
        return false;
    }
}

impl DeviceCompatibility for IosSimDevice {
    fn is_compatible_with_ios_platform(&self, platform: &IosPlatform) -> bool {
        platform.sim
            && (platform.toolchain.rustc_triple == "x86_64-apple-ios"
                || platform.toolchain.rustc_triple == "aarch64-apple-ios-sim")
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
        .output()?;
    let magic = String::from_utf8(magic.stdout)?;
    let target = magic
        .split(" ")
        .last()
        .ok_or_else(|| anyhow!("empty magic"))?;
    xcode::add_plist_to_app(&build_bundle, target, app_id)?;
    Ok(build_bundle)
}

fn launch_app(dev: &IosSimDevice, app_args: &[&str], _envs: &[&str]) -> Result<()> {
    use std::io::Write;
    let dir = ::tempdir::TempDir::new("mobiledevice-rs-lldb")?;
    let tmppath = dir.path();
    let mut install_path = String::from_utf8(
        process::Command::new("xcrun")
            .args(&["simctl", "get_app_container", &dev.id, "Dinghy"])
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
    let launch_output = process::Command::new("xcrun").args(&xcrun_args).output()?;
    let launch_output = String::from_utf8_lossy(&launch_output.stdout);

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
        panic!("Failed to get the exit status line from lldb: {:?}", output);
    }
}

fn launch_lldb_simulator(
    dev: &IosSimDevice,
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
        .status()?;
    if stat.success() {
        Ok(())
    } else {
        bail!("LLDB returned error code {:?}", stat.code())
    }
}
