use super::mobiledevice_sys::*;
use super::xcode;
use core_foundation::array::CFArray;
use core_foundation::base::{CFType, CFTypeRef, ItemRef, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::data::CFData;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_foundation_sys::number::kCFBooleanTrue;
use crate::device::make_remote_app_with_name;
use crate::errors::*;
use crate::ios::IosPlatform;
use libc::*;
use crate::project::Project;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fs;
use std::mem;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::ptr;
use std::thread;
use std::time::Duration;
use crate::Build;
use crate::BuildBundle;
use crate::Device;
use crate::DeviceCompatibility;
use crate::Runnable;

#[derive(Clone, Debug)]
pub struct IosDevice {
    pub id: String,
    pub name: String,
    ptr: *const am_device,
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
    pub fn new(ptr: *const am_device) -> Result<IosDevice> {
        let _session = ensure_session(ptr)?;
        let name = match device_read_value(ptr, "DeviceName")? {
            Some(Value::String(s)) => s,
            x => bail!("DeviceName should have been a string, was {:?}", x),
        };
        let cpu = match device_read_value(ptr, "CPUArchitecture")? {
            Some(Value::String(ref v)) if v == "arm64" => "aarch64",
            _ => "armv7",
        };
        let id = if let Value::String(id) = rustify(unsafe { AMDeviceCopyDeviceIdentifier(ptr) })? {
            id
        } else {
            bail!("unexpected id format")
        };
        Ok(IosDevice {
            ptr: ptr,
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
        install_app(self.ptr, &build_bundle.bundle_dir)?;
        Ok(build_bundle)
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
        _envs: &[&str],
    ) -> Result<BuildBundle> {
        let runnable = build
            .runnables
            .iter()
            .next()
            .ok_or_else(|| anyhow!("No executable compiled"))?;
        let build_bundle = self.install_app(project, build, runnable)?;
        let lldb_proxy = self.start_remote_lldb()?;
        run_remote(self.ptr, &lldb_proxy, &build_bundle.bundle_dir, args, true)?;
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
        _envs: &[&str],
    ) -> Result<Vec<BuildBundle>> {
        let mut build_bundles = vec![];
        for runnable in &build.runnables {
            let build_bundle = self.install_app(&project, &build, &runnable)?;
            let lldb_proxy = self.start_remote_lldb()?;
            run_remote(self.ptr, &lldb_proxy, &build_bundle.bundle_dir, args, false)?;
            build_bundles.push(build_bundle)
        }
        Ok(build_bundles)
    }

    fn start_remote_lldb(&self) -> Result<String> {
        let _ = ensure_session(self.ptr);
        let fd = start_remote_debug_server(self.ptr)?;
        debug!("start local lldb proxy");
        let proxy = start_lldb_proxy(fd)?;
        let url = format!("localhost:{}", proxy);
        debug!("started lldb proxy {}", url);
        Ok(url)
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
        _envs: &[&str],
    ) -> Result<BuildBundle> {
        let runnable = build
            .runnables
            .iter()
            .next()
            .ok_or_else(|| anyhow!("No executable compiled"))?;
        let build_bundle = self.install_app(project, build, runnable)?;
        let install_path = String::from_utf8(
            process::Command::new("xcrun")
                .args(&["simctl", "get_app_container", &self.id, "Dinghy"])
                .output()?
                .stdout,
        )?;
        launch_lldb_simulator(&self, &install_path, args, true)?;
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
        _envs: &[&str],
    ) -> Result<Vec<BuildBundle>> {
        let mut build_bundles = vec![];
        for runnable in &build.runnables {
            let build_bundle = self.install_app(&project, &build, &runnable)?;
            launch_app(&self, args)?;
            build_bundles.push(build_bundle);
        }
        Ok(build_bundles)
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
        platform.sim && platform.toolchain.rustc_triple == "x86_64-apple-ios"
    }
}

#[derive(Clone, Debug)]
enum Value {
    String(String),
    Data(Vec<u8>),
    I64(i64),
    Boolean(bool),
}

fn mk_result(rv: i32) -> Result<()> {
    if rv as u32 == 0xe80000e2 {
        bail!("error: Device is locked. ({:x})", rv)
    } else if rv as u32 == 0xe8000087 {
        bail!("error: 0xe8000087, Architecture mismatch")
    } else if rv as u32 == 0xe8008015 {
        bail!("error: 0xe8008015, A valid provisioning profile for this executable was not found.")
    } else if rv as u32 == 0xe8008016 {
        bail!("error: 0xe8008016, The executable was signed with invalid entitlements.")
    } else if rv as u32 == 0xe8008022 {
        bail!(
            "error: 0xe8000022, kAMDInvalidServiceError. (This one is relatively hard to diagnose. Try erasing the Dinghy app from the phone, rebooting the device, the computer, check for ios and xcode updates.)",
        )
    } else if rv as u32 == 0xe800007f {
        bail!("error: e800007f, The device OS version is too low.")
    } else if rv as u32 == 0xe8000007 {
        bail!("error: e8000007: Invalid argument.")
    } else if rv != 0 {
        bail!("error: {:x}", rv)
    } else {
        Ok(())
    }
}

fn rustify(raw: CFTypeRef) -> Result<Value> {
    unsafe {
        let cftype: CFType = TCFType::wrap_under_get_rule(mem::transmute(raw));
        if cftype.type_of() == CFString::type_id() {
            let value: CFString = TCFType::wrap_under_get_rule(mem::transmute(raw));
            return Ok(Value::String(value.to_string()));
        }

        if cftype.type_of() == CFData::type_id() {
            let value: CFData = TCFType::wrap_under_get_rule(mem::transmute(raw));
            return Ok(Value::Data(value.bytes().to_vec()));
        }
        if cftype.type_of() == CFNumber::type_id() {
            let value: CFNumber = TCFType::wrap_under_get_rule(mem::transmute(raw));
            if let Some(i) = value.to_i64() {
                return Ok(Value::I64(i));
            }
        }
        if cftype.type_of() == CFBoolean::type_id() {
            return Ok(Value::Boolean(raw == mem::transmute(kCFBooleanTrue)));
        }
        cftype.show();
        bail!("unknown value")
    }
}

fn device_read_value(dev: *const am_device, key: &str) -> Result<Option<Value>> {
    unsafe {
        let key = CFString::new(key);
        let raw = AMDeviceCopyValue(dev, ptr::null(), key.as_concrete_TypeRef());
        if raw.is_null() {
            return Ok(None);
        }
        Ok(Some(rustify(raw)?))
    }
}

fn xcode_dev_path() -> Result<PathBuf> {
    use std::process::Command;
    let command = Command::new("xcode-select").arg("-print-path").output()?;
    Ok(String::from_utf8(command.stdout)?.trim().into())
}

fn device_support_path(dev: *const am_device) -> Result<PathBuf> {
    let os_version = device_read_value(dev, "ProductVersion")?
        .ok_or_else(|| anyhow!("Could not get OS version"))?;
    if let Value::String(v) = os_version {
        platform_support_path("iPhoneOS.platform", &v)
    } else {
        bail!(
            "expected ProductVersion to be a String, found {:?}",
            os_version
        )
    }
}

fn platform_support_path(platform: &str, os_version: &str) -> Result<PathBuf> {
    let prefix = xcode_dev_path()?
        .join("Platforms")
        .join(platform)
        .join("DeviceSupport");
    debug!(
        "Looking for device support directory in {:?} for iOS version {:?}",
        prefix, os_version
    );
    let two_token_version: String = os_version
        .split(".")
        .take(2)
        .collect::<Vec<_>>()
        .join(".")
        .into();
    for directory in fs::read_dir(&prefix)? {
        let directory = directory?;
        let last = directory
            .file_name()
            .into_string()
            .map_err(|e| anyhow!("Could not parse {:?}", e))?;
        if last.starts_with(&two_token_version) {
            debug!("Picked {:?}", last);
            return Ok(prefix.join(directory.path()));
        }
    }
    bail!(
        "No device support directory for iOS version {} in {:?}. Time for an XCode \
         update?",
        two_token_version,
        prefix
    )
}

extern "C" fn mount_callback(_dict: CFDictionaryRef, _arg: *mut libc::c_void) {}

fn mount_developper_image(dev: *const am_device) -> Result<()> {
    unsafe {
        let _session = ensure_session(dev);
        let ds_path = device_support_path(dev)?;
        let image_path = ds_path.join("DeveloperDiskImage.dmg");
        debug!("Developper image path: {:?}", image_path);
        let sig_image_path = ds_path.join("DeveloperDiskImage.dmg.signature");
        let sig = fs::read(sig_image_path)?;
        let sig = CFData::from_buffer(&sig);

        let options = [
            (
                CFString::from_static_string("ImageType"),
                CFString::from_static_string("Developper").as_CFType(),
            ),
            (
                CFString::from_static_string("ImageSignature"),
                sig.as_CFType(),
            ),
        ];
        let options = CFDictionary::from_CFType_pairs(&options);
        let r = AMDeviceMountImage(
            dev,
            CFString::new(image_path.to_str().unwrap()).as_concrete_TypeRef(),
            options.as_concrete_TypeRef(),
            mount_callback,
            0,
        );
        debug!("AMDeviceMountImage returns: {:x}", r);
        if r as u32 == 0xe8000076 {
            debug!("Error, already mounted, going on");
            return Ok(());
        }
        mk_result(r)?;
        Ok(())
    }
}

fn make_ios_app(
    project: &Project,
    build: &Build,
    runnable: &Runnable,
    app_id: &str,
) -> Result<BuildBundle> {
    use crate::project;
    let build_bundle = make_remote_app_with_name(project, build, runnable, Some("Dinghy.app"))?;
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

struct Session(*const am_device);

fn ensure_session(dev: *const am_device) -> Result<Session> {
    unsafe {
        mk_result(AMDeviceConnect(dev))?;
        if AMDeviceIsPaired(dev) == 0 {
            bail!("lost pairing")
        };
        mk_result(AMDeviceValidatePairing(dev))?;
        mk_result(AMDeviceStartSession(dev))?;
        Ok(Session(dev))
        // debug!("ensure session 4 ({:x})", rv);
        // if rv as u32 == 0xe800001d {
        // Ok(Session(::std::ptr::null()))
        // } else {
        // mk_result(rv)?;
        // Ok(Session(dev))
        // }
        //
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                if let Err(e) = mk_result(AMDeviceStopSession(self.0)) {
                    debug!("Error closing session {:?}", e);
                }
                if let Err(e) = mk_result(AMDeviceDisconnect(self.0)) {
                    error!("Error disconnecting {:?}", e);
                }
            }
        }
    }
}

pub fn install_app<P: AsRef<Path>>(dev: *const am_device, app: P) -> Result<()> {
    unsafe {
        let _session = ensure_session(dev)?;
        let path = app
            .as_ref()
            .to_str()
            .ok_or_else(|| anyhow!("failure to convert {:?}", app.as_ref()))?;
        let url =
            ::core_foundation::url::CFURL::from_file_system_path(CFString::new(path), 0, true);
        let options = [(
            CFString::from_static_string("PackageType"),
            CFString::from_static_string("Developper").as_CFType(),
        )];
        let options = CFDictionary::from_CFType_pairs(&options);
        mk_result(AMDeviceSecureTransferPath(
            0,
            dev,
            url.as_concrete_TypeRef(),
            options.as_concrete_TypeRef(),
            ptr::null(),
            ptr::null(),
        ))?;
        mk_result(AMDeviceSecureInstallApplication(
            0,
            dev,
            url.as_concrete_TypeRef(),
            options.as_concrete_TypeRef(),
            ptr::null(),
            ptr::null(),
        ))?;
    }
    Ok(())
}

fn start_remote_debug_server(dev: *const am_device) -> Result<c_int> {
    unsafe {
        debug!("mount developper image");
        mount_developper_image(dev)?;
        debug!("start debugserver on phone");
        let _session = ensure_session(dev)?;
        let mut handle: *const c_void = std::ptr::null();
        mk_result(AMDeviceSecureStartService(
            dev,
            CFString::from_static_string("com.apple.debugserver").as_concrete_TypeRef(),
            ptr::null_mut(),
            &mut handle,
        ))?;
        debug!("debug server running");

        let fd = AMDServiceConnectionGetSocket(handle);
        Ok(fd)
    }
}

fn start_lldb_proxy(fd: c_int) -> Result<u16> {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::os::unix::io::FromRawFd;
    let device = unsafe { TcpStream::from_raw_fd(fd) };
    let proxy = TcpListener::bind("127.0.0.1:0")?;
    let addr = proxy.local_addr()?;
    device.set_nonblocking(true)?;
    thread::spawn(move || {
        fn server(proxy: TcpListener, mut device: TcpStream) -> Result<()> {
            for stream in proxy.incoming() {
                let mut stream = stream.expect("Failure while accepting connection");
                stream.set_nonblocking(true)?;
                let mut buffer = [0; 16384];
                loop {
                    if let Ok(n) = device.read(&mut buffer) {
                        if n == 0 {
                            break;
                        }
                        stream.write_all(&buffer[0..n])?;
                    } else if let Ok(n) = stream.read(&mut buffer) {
                        if n == 0 {
                            break;
                        }
                        device.write_all(&buffer[0..n])?;
                    } else {
                        thread::sleep(Duration::new(0, 100));
                    }
                }
            }
            Ok(())
        }
        server(proxy, device).unwrap();
    });
    Ok(addr.port())
}

fn launch_lldb_device<P: AsRef<Path>, P2: AsRef<Path>>(
    dev: *const am_device,
    proxy: &str,
    local: P,
    remote: P2,
    args: &[&str],
    debugger: bool,
) -> Result<()> {
    use std::io::Write;
    use std::process::Command;
    let _session = ensure_session(dev);
    let dir = ::tempdir::TempDir::new("mobiledevice-rs-lldb")?;
    let tmppath = dir.path();
    let lldb_script_filename = tmppath.join("lldb-script");
    let sysroot = device_support_path(dev)?
        .to_str()
        .ok_or_else(|| anyhow!("could not read sysroot"))?
        .to_owned();
    {
        let python_lldb_support = tmppath.join("helpers.py");
        fs::File::create(&python_lldb_support)?.write_all(include_bytes!("helpers.py"))?;
        let mut script = fs::File::create(&lldb_script_filename)?;
        writeln!(script, "platform select remote-ios --sysroot '{}'", sysroot)?;
        writeln!(
            script,
            "target create {}",
            local
                .as_ref()
                .to_str()
                .ok_or_else(|| anyhow!("untranslatable path"))?
        )?;
        writeln!(script, "script pass")?;

        writeln!(script, "command script import {:?}", python_lldb_support)?;
        writeln!(
            script,
            "command script add -f helpers.set_remote_path set_remote_path"
        )?;
        writeln!(
            script,
            "command script add -f helpers.connect_command connect"
        )?;
        writeln!(
            script,
            "command script add -s synchronous -f helpers.start start"
        )?;

        writeln!(script, "connect connect://{}", proxy)?;
        writeln!(
            script,
            "set_remote_path {}",
            remote.as_ref().to_str().unwrap()
        )?;
        if !debugger {
            writeln!(script, "start {}", args.join(" "))?;
            writeln!(script, "quit")?;
        }
    }

    let stat = Command::new("lldb")
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

fn launch_app(dev: &IosSimDevice, app_args: &[&str]) -> Result<()> {
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
    debug!("LLDB OUTPUT: {}", output);
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
    //
    // Executable module set to .....
    // Architecture set to: x86_64h-apple-ios-.
    // (lldb) continue
    // Process 34163 resuming
    // Process 34163 exited with status = 101 (0x00000065)
    //
    // (lldb) quit
    //
    // We need the "exit with status" line which is the 3rd from the last
    let lines: Vec<&str> = output.lines().rev().collect();
    let exit_status_line = lines.get(2);
    if let Some(exit_status_line) = exit_status_line {
        let words: Vec<&str> = exit_status_line.split_whitespace().rev().collect();
        if let Some(exit_status) = words.get(1) {
            let exit_status = exit_status.parse::<u32>()?;
            if exit_status == 0 {
                Ok(())
            } else {
                panic!("Non-zero exit code from lldb: {}", exit_status);
            }
        } else {
            panic!(
                "Failed to parse lldb exit line for an exit status. {:?}",
                words
            );
        }
    } else {
        panic!("Failed to get the exit status line from lldb: {:?}", lines);
    }
}

fn launch_lldb_simulator(
    dev: &IosSimDevice,
    installed: &str,
    args: &[&str],
    debugger: bool,
) -> Result<()> {
    use std::io::Write;
    use std::process::Command;
    let dir = ::tempdir::TempDir::new("mobiledevice-rs-lldb")?;
    let tmppath = dir.path();
    let lldb_script_filename = tmppath.join("lldb-script");
    {
        let python_lldb_support = tmppath.join("helpers.py");
        fs::File::create(&python_lldb_support)?.write_all(include_bytes!("helpers.py"))?;
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

pub fn run_remote<P: AsRef<Path>>(
    dev: *const am_device,
    lldb_proxy: &str,
    app_path: P,
    args: &[&str],
    debugger: bool,
) -> Result<()> {
    let _session = ensure_session(dev)?;
    let plist = plist::Value::from_file(app_path.as_ref().join("Info.plist"))?;
    let bundle_id = plist
        .as_dictionary()
        .and_then(|btreemap| btreemap.get("CFBundleIdentifier"))
        .and_then(|bi| bi.as_string())
        .expect("failed to read CFBundleIdentifier");

    let options = [(
        CFString::from_static_string("ReturnAttributes"),
        CFArray::from_CFTypes(&[
            CFString::from_static_string("CFBundleIdentifier"),
            CFString::from_static_string("Path"),
        ]),
    )];
    let options = CFDictionary::from_CFType_pairs(&options);
    let apps: CFDictionaryRef = ptr::null();
    unsafe {
        mk_result(AMDeviceLookupApplications(
            dev,
            options.as_concrete_TypeRef(),
            std::mem::transmute(&apps),
        ))?;
    }
    let apps: CFDictionary<CFString, CFDictionary<CFString, CFTypeRef>> =
        unsafe { TCFType::wrap_under_get_rule(apps) };
    let app_info: ItemRef<CFDictionary<CFString, CFTypeRef>> =
        apps.get(CFString::new(bundle_id).as_concrete_TypeRef());
    let remote: String = if let Ok(Value::String(remote)) =
        rustify(*app_info.get(CFString::from_static_string("Path")))
    {
        remote
    } else {
        bail!("Invalid info")
    };
    launch_lldb_device(dev, lldb_proxy, app_path, remote, args, debugger)?;
    Ok(())
}

#[allow(dead_code)]
fn properties(dev: *const am_device) -> Result<HashMap<&'static str, Value>> {
    let properties = [
        "ActivationPublicKey",
        "ActivationState",
        "ActivationStateAcknowledged",
        "ActivityURL",
        "BasebandBootloaderVersion",
        "BasebandSerialNumber",
        "BasebandStatus",
        "BasebandVersion",
        "BluetoothAddress",
        "BuildVersion",
        "CPUArchitecture",
        "DeviceCertificate",
        "DeviceClass",
        "DeviceColor",
        "DeviceName",
        "DevicePublicKey",
        "DieID",
        "FirmwareVersion",
        "HardwareModel",
        "HardwarePlatform",
        "HostAttached",
        "IMLockdownEverRegisteredKey",
        "IntegratedCircuitCardIdentity",
        "InternationalMobileEquipmentIdentity",
        "InternationalMobileSubscriberIdentity",
        "iTunesHasConnected",
        "MLBSerialNumber",
        "MobileSubscriberCountryCode",
        "MobileSubscriberNetworkCode",
        "ModelNumber",
        "PartitionType",
        "PasswordProtected",
        "PhoneNumber",
        "ProductionSOC",
        "ProductType",
        "ProductVersion",
        "ProtocolVersion",
        "ProximitySensorCalibration",
        "RegionInfo",
        "SBLockdownEverRegisteredKey",
        "SerialNumber",
        "SIMStatus",
        "SoftwareBehavior",
        "SoftwareBundleVersion",
        "SupportedDeviceFamilies",
        "TelephonyCapability",
        "TimeIntervalSince1970",
        "TimeZone",
        "TimeZoneOffsetFromUTC",
        "TrustedHostAttached",
        "UniqueChipID",
        "UniqueDeviceID",
        "UseActivityURL",
        "UseRaptorCerts",
        "Uses24HourClock",
        "WeDelivered",
        "WiFiAddress",
    ];
    let mut props = HashMap::new();
    for p in properties.iter() {
        if let Some(v) = device_read_value(dev, p)? {
            props.insert(*p, v);
        }
    }
    Ok(props)
}
