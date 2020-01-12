use libc::c_void;
use std::{mem, ptr, sync, thread};

pub use self::device::{IosDevice, IosSimDevice};
use self::mobiledevice_sys::*;
pub use self::platform::IosPlatform;
use {Compiler, Device, Platform, PlatformManager, Result};

mod device;
mod mobiledevice_sys;
mod platform;
mod xcode;

#[derive(Debug, Clone)]
pub struct SignatureSettings {
    pub identity: SigningIdentity,
    pub file: String,
    pub entitlements: String,
    pub name: String,
    pub profile: String,
}

#[derive(Debug, Clone)]
pub struct SigningIdentity {
    pub id: String,
    pub name: String,
    pub team: String,
}

pub struct IosManager {
    compiler: sync::Arc<Compiler>,
    devices: sync::Arc<sync::Mutex<Vec<IosDevice>>>,
}

impl IosManager {
    pub fn new(compiler: sync::Arc<Compiler>) -> Result<Option<IosManager>> {
        let devices = sync::Arc::new(sync::Mutex::new(vec![]));

        let devices_to_take_away = Box::new(devices.clone());
        thread::spawn(move || {
            let notify: *const am_device_notification = ptr::null();
            unsafe {
                AMDeviceNotificationSubscribe(
                    device_callback,
                    0,
                    0,
                    Box::into_raw(devices_to_take_away) as *mut c_void,
                    &mut notify.into(),
                );
            }
            ::core_foundation::runloop::CFRunLoop::run_current();
        });

        extern "C" fn device_callback(
            info: *mut am_device_notification_callback_info,
            devices: *mut c_void,
        ) {
            let device: *const am_device = unsafe { (*info).dev };
            let devices: &sync::Arc<sync::Mutex<Vec<IosDevice>>> =
                unsafe { mem::transmute(devices) };
            // FIXME: unwrap -> panic in FFI callback
            let _ = devices
                .lock()
                .map(|mut devices| devices.push(IosDevice::new(device).unwrap()));
        }

        Ok(Some(IosManager {
            devices: devices,
            compiler,
        }))
    }
}

impl PlatformManager for IosManager {
    fn devices(&self) -> Result<Vec<Box<dyn Device>>> {
        let sims_list = ::std::process::Command::new("xcrun")
            .args(&["simctl", "list", "--json", "devices"])
            .output()?;
        if !sims_list.status.success() {
            info!(
                "Failed while looking for ios simulators. It this is not expected, you need to make sure `xcrun simctl list --json` works."
            );
            return Ok(vec![]);
        }
        let sims_list = String::from_utf8(sims_list.stdout)?;
        let sims_list = ::json::parse(&sims_list)?;
        let mut sims: Vec<Box<dyn Device>> = vec![];
        for (ref k, ref v) in sims_list["devices"].entries() {
            for ref sim in v.members() {
                if sim["state"] == "Booted" {
                    sims.push(Box::new(IosSimDevice {
                        name: sim["name"]
                            .as_str()
                            .ok_or("unexpected simulator list format (missing name)")?
                            .to_string(),
                        id: sim["udid"]
                            .as_str()
                            .ok_or("unexpected simulator list format (missing udid)")?
                            .to_string(),
                        os: k.split(" ").last().unwrap().to_string(),
                    }))
                }
            }
        }
        let devices = self.devices.lock().map_err(|_| "poisoned lock")?;
        Ok(devices
            .iter()
            .map(|d| Box::new(d.clone()) as Box<dyn Device>)
            .chain(sims.into_iter())
            .collect())
    }

    fn platforms(&self) -> Result<Vec<Box<dyn Platform>>> {
        ["armv7", "armv7s", "aarch64", "i386", "x86_64"]
            .iter()
            .map(|arch| {
                let id = format!("auto-ios-{}", arch);
                let rustc_triple = format!("{}-apple-ios", arch);
                IosPlatform::new(
                    id,
                    &rustc_triple,
                    &self.compiler,
                    ::config::PlatformConfiguration::default(),
                )
                .map(|pf| pf as Box<dyn Platform>)
            })
            .collect()
    }
}
