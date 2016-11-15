#![allow(non_camel_case_types)]
extern crate core_foundation as cf;
extern crate core_foundation_sys;
#[macro_use]
extern crate error_chain;
extern crate libc;
extern crate tempdir;

extern crate mobiledevice_sys as mds;

mod errors;

use std::thread;
use std::time::Duration;
use std::fs;
use std::path;
use std::ptr;

use cf::base::{ CFType, CFTypeRef, TCFType };
use cf::string::CFString;
use cf::dictionary::{ CFDictionary, CFDictionaryRef };

use libc::*;

use errors::*;

macro_rules! mk_result {
    ($e:expr) => {{
        let rv = $e;
        if rv != 0 {
            Err(format!("error: {:x}", rv))
        } else {
            Ok(())
        }
    }}
}

fn main() {
    println!("start loop");
    thread::spawn(device_thread);

    thread::sleep(Duration::from_secs(1200));
    println!("ok, enough");
}

fn device_thread() {
    let notify: *const mds::am_device_notification = std::ptr::null();
    unsafe {
        mds::AMDeviceNotificationSubscribe(device_callback, 0, 0, ptr::null(), &mut notify.into());
    }
    cf::runloop::CFRunLoop::run_current();
}

#[derive(Clone,Debug)]
enum Value {
    String(String),
    Data(Vec<u8>),
    I64(i64),
    Boolean(bool),
}

fn rustify(raw: CFTypeRef) -> Result<Value> {
    use cf::base::TCFType;
    use cf::data::CFData;
    use cf::number::CFNumber;
    use cf::boolean::CFBoolean;
    use core_foundation_sys::number::kCFBooleanTrue;

    unsafe {
        let cftype: CFType = TCFType::wrap_under_get_rule(std::mem::transmute(raw));
        if cftype.type_of() == CFString::type_id() {
            let value: CFString = TCFType::wrap_under_get_rule(std::mem::transmute(raw));
            return Ok(Value::String(value.to_string()));
        }

        if cftype.type_of() == CFData::type_id() {
            let value: CFData = TCFType::wrap_under_get_rule(std::mem::transmute(raw));
            return Ok(Value::Data(value.bytes().to_vec()));
        }
        if cftype.type_of() == CFNumber::type_id() {
            let value: CFNumber = TCFType::wrap_under_get_rule(std::mem::transmute(raw));
            if let Some(i) = value.to_i64() {
                return Ok(Value::I64(i));
            }
        }
        if cftype.type_of() == CFBoolean::type_id() {
            return Ok(Value::Boolean(raw == std::mem::transmute(kCFBooleanTrue)));
        }
        cftype.show();
        Err("unknown value")?
    }
}

fn device_read_value(dev: *const mds::am_device, key: &str) -> Result<Option<Value>> {
    unsafe {
        let key = CFString::new(key);
        let raw = mds::AMDeviceCopyValue(dev, ptr::null(), key.as_concrete_TypeRef());
        if raw.is_null() {
            return Ok(None);
        }
        Ok(Some(rustify(raw)?))
    }
}

fn xcode_dev_path() -> Result<path::PathBuf> {
    use std::process::Command;
    let command = Command::new("xcode-select").arg("-print-path").output()?;
    Ok(String::from_utf8(command.stdout)?.trim().into())
}

fn device_support_path(dev: *const mds::am_device) -> Result<Option<path::PathBuf>> {
    let prefix = xcode_dev_path()?.join("Platforms/iPhoneOS.platform/DeviceSupport");
    let os_version = device_read_value(dev, "ProductVersion")?.ok_or("Could not get OS version")?;
    let two_token_version: String = if let Value::String(v) = os_version {
        v.split(".").take(2).collect::<Vec<_>>().join(".").into()
    } else {
        Err("ProductVersion should have be a string")?
    };
    for directory in fs::read_dir(&prefix)? {
        let directory = directory?;
        let last = directory.file_name()
            .into_string()
            .map_err(|d| format!("Could not parse {:?}", d))?;
        if last.starts_with(&two_token_version) {
            return Ok(Some(prefix.join(directory.path())));
        }
    }
    Ok(None)
}

fn mount_developper_image(dev: *const mds::am_device) -> Result<()> {
    use std::io::Read;
    unsafe {
        let ds_path = device_support_path(dev)?.ok_or("No device support found in xcode")?;
        let image_path = ds_path.join("DeveloperDiskImage.dmg");
        let sig_image_path = ds_path.join("DeveloperDiskImage.dmg.signature");
        let mut sig: Vec<u8> = vec![];
        fs::File::open(sig_image_path)?.read_to_end(&mut sig);
        let sig = cf::data::CFData::from_buffer(&*sig);

        use cf::dictionary::CFDictionary;
        use cf::string::CFString;
        let options = [(CFString::from_static_string("ImageType"),
                        CFString::from_static_string("Developper").as_CFType()),
                       (CFString::from_static_string("ImageSignature"), sig.as_CFType())];
        let options = CFDictionary::from_CFType_pairs(&options);
        let r = mds::AMDeviceMountImage(dev,
                           CFString::new(image_path.to_str().unwrap()).as_concrete_TypeRef(),
                           options.as_concrete_TypeRef(),
                           mount_callback,
                           0);
        if r == 0xe8000076 { // already mounted, that's fine.
            return Ok(());
        }
        mk_result!(r)?;
        Ok(())
    }
}

extern "C" fn mount_callback(dict: CFDictionaryRef, _arg: *mut c_void) {
    let status = unsafe {
        let cft: CFDictionary = TCFType::wrap_under_get_rule(dict);
        let status = cft.get(CFString::from_static_string("Status").as_CFTypeRef());
        if let Ok(Value::String(r)) = rustify(status) {
            print!("{}       \r", r);
        }
        ()
    };

}

fn start_remote_debug_server(dev: *const mds::am_device, url: &str) -> Result<c_int> {
    unsafe {
        mk_result!(mds::AMDeviceConnect(dev))?;
        println!("Pairing ? {}", mds::AMDeviceIsPaired(dev));
        mk_result!(mds::AMDeviceValidatePairing(dev))?;
        println!("Start Session...");
        mk_result!(mds::AMDeviceStartSession(dev))?;
        println!("Mount image...");
        mount_developper_image(dev)?;
        println!("Start debug service...");
        let mut fd: c_int = 0;
        mk_result!(mds::AMDeviceStartService(dev,
                                        CFString::from_static_string("com.apple.debugserver")
                                            .as_concrete_TypeRef(),
                                        &mut fd,
                                        ptr::null()))?;
        Ok(fd)
    }
}

fn start_lldb_proxy(fd: c_int) -> Result<u16> {
    use std::net::{ TcpStream, TcpListener};
    use std::os::unix::io::FromRawFd;
    use std::io::{Read, Write};
    let device = unsafe { TcpStream::from_raw_fd(fd) };
    let proxy = TcpListener::bind("127.0.0.1:0")?;
    let addr = proxy.local_addr()?;
    device.set_nonblocking(true)?;
    println!("listening on {:?}", addr);
    thread::spawn(move || {
        fn server(proxy:TcpListener, mut device:TcpStream) -> Result<()> {
            for stream in proxy.incoming() {
                let mut stream = stream.expect("Failure while accepting connection");
                stream.set_nonblocking(true)?;
                let mut buffer = [0; 1024];
                loop {
                    if let Ok(n) = stream.read(&mut buffer) {
                        if n == 0 {
                            break;
                        }
                        device.write_all(&buffer[0..n])?;
                    } else if let Ok(n) = device.read(&mut buffer) {
                        if n == 0 {
                            break;
                        }
                        stream.write_all(&buffer[0..n])?;
                    } else {
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            }
            Ok(())
        }
        server(proxy, device).unwrap();
    });
    Ok(addr.port())
}

fn launch_lldb(dev: *const mds::am_device, proxy_port:u16) -> Result<()> {
    use std::process::Command;
    use std::io::Write;
    let dir = tempdir::TempDir::new("mobiledevice-rs-lldb")?;
    let tmppath = dir.into_path();//FIXME
    let lldb_script_filename = tmppath.join("lldb-script");
    let sysroot = device_support_path(dev)?.ok_or("no sysroot ?")?.to_str().ok_or("could not read sysroot")?.to_owned();
    {
        let python_lldb_support = tmppath.join("helpers.py");
        fs::File::create(&python_lldb_support)?.write_all(include_bytes!("helpers.py"))?;
        let mut script = fs::File::create(&lldb_script_filename)?;
        writeln!(script, "platform select remote-ios --sysroot '{}'", sysroot)?;
        writeln!(script, "target create /Users/kali/dev/run-rust-on-ios/Command.app")?;
        writeln!(script, "script pass")?;

        writeln!(script, "command script import {:?}", python_lldb_support)?;
        writeln!(script, "command script add -f helpers.set_remote_path set_remote_path")?;
        writeln!(script, "command script add -f helpers.connect_command connect")?;
        writeln!(script, "command script add -s synchronous -f helpers.run_command run")?;

        writeln!(script, "connect connect://127.0.0.1:{}", proxy_port)?;
        writeln!(script, "set_remote_path /private/var/containers/Bundle/Application/F595084A-70AA-40E0-9AB2-5516EABEA648/Command.app")?;
        writeln!(script, "run")?;
        writeln!(script, "quit")?;
    }

    Command::new("lldb").arg("-Q").arg("-s").arg(lldb_script_filename).status()?;
    Ok(())
}

fn run_remote(dev: *const mds::am_device) -> Result<()> {
    let fd = start_remote_debug_server(dev, "")?;
    let proxy = start_lldb_proxy(fd)?;
    launch_lldb(dev, proxy)?;
    Ok(())
}

extern "C" fn device_callback(info: *mut mds::am_device_notification_callback_info, _arg: *mut c_void) {
    let device = unsafe { (*info).dev };
    unsafe { mds::AMDeviceConnect(device) };
    /*
    let properties = ["ActivationPublicKey",
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
                      "WiFiAddress"];
    for p in properties.iter() {
        if let Ok(Some(v)) = device_read_value(device, p) {
            println!("{}\t{:?}", p, v);
        }
    }
    */
    run_remote(device).unwrap_or_else(|e| println!("{}", e));
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
