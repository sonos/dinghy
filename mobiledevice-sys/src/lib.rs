#![allow(non_camel_case_types)]
extern crate core_foundation;
extern crate core_foundation_sys;

extern crate libc;
use libc::*;

use core_foundation::dictionary::CFDictionaryRef;
use core_foundation::string::CFStringRef;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct am_device {
    unknown0: [c_char; 16],
    device_id: c_int,
    product_id: c_int,
    serial: *const c_char,
    unknown1: c_int,
    unknown2: [c_char; 4],
    lockdown_conn: c_int,
    unknown3: [c_char; 8],
}

unsafe impl Send for am_device {}

pub const ADNCI_MSG_CONNECTED: c_uint = 1;
pub const ADNCI_MSG_DISCONNECTED: c_uint = 2;
pub const ADNCI_MSG_UNSUBSCRIBED: c_uint = 3;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct am_device_notification_callback_info {
    pub dev: *mut am_device,
    pub msg: ::std::os::raw::c_uint,
    pub subscription: *mut am_device_notification,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct am_device_notification {
    unknown0: c_int,
    unknown1: c_int,
    unknown2: c_int,
    callback: *const am_device_notification_callback,
    unknown3: c_int,
}

pub type am_device_notification_callback = extern "C" fn(*mut am_device_notification_callback_info,
                                                         *mut c_void);


pub type am_device_mount_callback = extern "C" fn(CFDictionaryRef, *mut c_void);
#[link(name = "MobileDevice", kind = "framework")]
extern "C" {
    pub fn AMDeviceNotificationSubscribe(callback: am_device_notification_callback,
                                         unused0: c_uint,
                                         unused1: c_uint,
                                         dn_unknown3: *const c_void,
                                         notification: *mut *const am_device_notification)
                                         -> c_int;
    pub fn AMDeviceCopyValue(device: *const am_device,
                             domain: CFStringRef,
                             cfstring: CFStringRef)
                             -> *const c_void;
    pub fn AMDeviceConnect(device: *const am_device) -> c_int;
    pub fn AMDeviceDisconnect(device: *const am_device) -> c_int;

    pub fn AMDeviceIsPaired(device: *const am_device) -> c_int;
    pub fn AMDeviceValidatePairing(device: *const am_device) -> c_int;
    pub fn AMDeviceStartSession(device: *const am_device) -> c_int;
    pub fn AMDeviceStopSession(device: *const am_device) -> c_int;

    pub fn AMDeviceMountImage(device: *const am_device,
                              image: CFStringRef,
                              options: CFDictionaryRef,
                              callback: am_device_mount_callback,
                              cbarg: c_int)
                              -> c_int;

    pub fn AMDeviceStartService(device: *const am_device,
                                service_name: CFStringRef,
                                socket_fd: *mut c_int,
                                unknown: *const c_int)
                                -> c_int;
}
