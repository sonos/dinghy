use std::collections::HashMap;
use std::fmt::Display;

pub use self::device::{AppleSimDevice, IosDevice};
pub use self::platform::AppleDevicePlatform;
use crate::{Device, Platform, PlatformManager, Result};
use itertools::Itertools;

mod device;
mod platform;
mod xcode;

use anyhow::{anyhow, bail, Context};
use log::info;

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

#[derive(Debug, Clone, PartialEq)]
pub enum AppleSimulatorType {
    Ios,
    Watchos,
    Tvos,
}
impl Display for AppleSimulatorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let val = match self {
            AppleSimulatorType::Ios => "ios",
            AppleSimulatorType::Watchos => "watchos",
            AppleSimulatorType::Tvos => "tvos",
        };
        f.write_str(val)
    }
}

pub struct IosManager {
    devices: Vec<Box<dyn Device>>,
}

impl IosManager {
    pub fn new() -> Result<Option<IosManager>> {
        let devices = devices()
            .context("Could not list iOS devices")?
            .into_iter()
            .chain(
                simulators(AppleSimulatorType::Ios)
                    .context("Could not list iOS simulators")?
                    .into_iter(),
            )
            .collect();
        Ok(Some(IosManager { devices }))
    }
}

impl PlatformManager for IosManager {
    fn devices(&self) -> Result<Vec<Box<dyn Device>>> {
        Ok(self.devices.clone())
    }

    fn platforms(&self) -> Result<Vec<Box<dyn Platform>>> {
        [
            "armv7",
            "armv7s",
            "aarch64",
            "i386",
            "x86_64",
            "aarch64-sim",
        ]
        .iter()
        .map(|arch| {
            let id = format!("auto-ios-{}", arch);
            let rustc_triple = if *arch != "aarch64-sim" {
                format!("{}-apple-ios", arch)
            } else {
                format!("aarch64-apple-ios-sim")
            };

            let simulator = if *arch == "x86_64" || *arch == "aarch64-sim" {
                Some(AppleSimulatorType::Ios)
            } else {
                None
            };

            AppleDevicePlatform::new(
                id,
                &rustc_triple,
                simulator,
                crate::config::PlatformConfiguration::default(),
            )
            .map(|pf| pf as Box<dyn Platform>)
        })
        .collect()
    }
}

pub struct WatchosManager {
    devices: Vec<Box<dyn Device>>,
}

impl WatchosManager {
    pub fn new() -> Result<Option<Self>> {
        let devices = simulators(AppleSimulatorType::Watchos)?;
        Ok(Some(Self { devices }))
    }
}
impl PlatformManager for WatchosManager {
    fn devices(&self) -> Result<Vec<Box<dyn Device>>> {
        Ok(self.devices.clone())
    }

    fn platforms(&self) -> Result<Vec<Box<dyn Platform>>> {
        ["arm64_32", "aarch64", "x86_64-sim", "aarch64-sim"]
            .iter()
            .map(|arch| {
                let id = format!("auto-watchos-{}", arch);

                // Apple watch simulator targets are x86_64-apple-watchos-sim or
                // aarch64-apple-watchos-sim
                let rustc_triple = if *arch == "aarch64-sim" {
                    format!("aarch64-apple-watchos-sim")
                } else if *arch == "x86_64-sim" {
                    format!("x86_64-apple-watchos-sim")
                } else {
                    format!("{}-apple-watchos", arch)
                };
                let simulator = if *arch == "x86_64-sim" || *arch == "aarch64-sim" {
                    Some(AppleSimulatorType::Watchos)
                } else {
                    None
                };

                AppleDevicePlatform::new(
                    id,
                    &rustc_triple,
                    simulator,
                    crate::config::PlatformConfiguration::default(),
                )
                .map(|pf| pf as Box<dyn Platform>)
            })
            .collect()
    }
}

pub struct TvosManager {
    devices: Vec<Box<dyn Device>>,
}

impl TvosManager {
    pub fn new() -> Result<Option<Self>> {
        let devices = simulators(AppleSimulatorType::Tvos)?;
        Ok(Some(Self { devices }))
    }
}

impl PlatformManager for TvosManager {
    fn devices(&self) -> Result<Vec<Box<dyn Device>>> {
        Ok(self.devices.clone())
    }

    fn platforms(&self) -> Result<Vec<Box<dyn Platform>>> {
        ["aarch64", "x86_64", "aarch64-sim"]
            .iter()
            .map(|arch| {
                let id = format!("auto-tvos-{}", arch);
                let rustc_triple = if *arch != "aarch64-sim" {
                    format!("{}-apple-tvos", arch)
                } else {
                    format!("aarch64-apple-tvos-sim")
                };
                let simulator = if *arch == "x86_64" || *arch == "aarch64-sim" {
                    Some(AppleSimulatorType::Tvos)
                } else {
                    None
                };
                AppleDevicePlatform::new(
                    id,
                    &rustc_triple,
                    simulator,
                    crate::config::PlatformConfiguration::default(),
                )
                .map(|pf| pf as Box<dyn Platform>)
            })
            .collect()
    }
}

fn simulators(sim_type: AppleSimulatorType) -> Result<Vec<Box<dyn Device>>> {
    let sims_list = ::std::process::Command::new("xcrun")
        .args(&[
            "simctl",
            "list",
            "--json",
            "devices",
            sim_type.to_string().as_str(),
        ])
        .output()?;
    if !sims_list.status.success() {
        info!(
            "Failed while looking for ios simulators. It this is not expected, you need to make sure `xcrun simctl list --json` works."
            );
        return Ok(vec![]);
    }
    let sims_list = String::from_utf8(sims_list.stdout)?;
    let sims_list = json::parse(&sims_list)
        .with_context(|| "Could not parse output for: `xcrun simctl list --json devices` as json. Please try to make this command work and retry.")?;
    let mut sims: Vec<Box<dyn Device>> = vec![];
    for (ref k, ref v) in sims_list["devices"].entries() {
        for ref sim in v.members() {
            if sim["state"] == "Booted" {
                sims.push(Box::new(AppleSimDevice {
                    name: sim["name"]
                        .as_str()
                        .ok_or_else(|| anyhow!("unexpected simulator list format (missing name)"))?
                        .to_string(),
                    id: sim["udid"]
                        .as_str()
                        .ok_or_else(|| anyhow!("unexpected simulator list format (missing udid)"))?
                        .to_string(),
                    os: k.split(" ").last().unwrap().to_string(),
                    sim_type: sim_type.clone(),
                }))
            }
        }
    }
    Ok(sims)
}

fn devices() -> Result<Vec<Box<dyn Device>>> {
    let mut devices: HashMap<String, IosDevice> = Default::default();
    devices_from_devicectl(&mut devices)?;
    devices_from_ios_deploy(&mut devices)?;
    Ok(devices
        .into_values()
        .map(|d| Box::new(d) as _)
        .collect_vec())
}

fn devices_from_devicectl(devices: &mut HashMap<String, IosDevice>) -> Result<()> {
    let tempdir = tempdir::TempDir::new("dinghy-ios")?;
    let tmpjson = tempdir.path().join("json");
    let devicectl = std::process::Command::new("xcrun")
        .args("devicectl list devices --quiet --json-output".split_whitespace().collect_vec())
        .arg(&tmpjson)
        .stderr(std::process::Stdio::inherit())
        .output()
        .context("Failed to launch xcrun command. Please check that \"xcrun devicectl list devices\" works")?;
    if !devicectl.status.success() {
        bail!("xcrun command failed. Please check that \"xcrun devicectl list devices\" works.\n{devicectl:?}");
    }
    for device in json::parse(&std::fs::read_to_string(tmpjson)?)?["result"]["devices"].members() {
        let udid = device["hardwareProperties"]["udid"]
            .as_str()
            .context("no identifier in device json")?
            .to_string();
        let device = IosDevice::new(
            device["deviceProperties"]["name"]
                .as_str()
                .context("no name in device json")?
                .to_string(),
            udid.clone(),
            device["hardwareProperties"]["cpuType"]["name"]
                .as_str()
                .context("no cpuType in device json")?,
            device["deviceProperties"]["osVersionNumber"]
                .as_str()
                .context("no osVersionNumber")?
                .to_string(),
        )?;
        devices.insert(udid, device);
    }
    Ok(())
}

fn devices_from_ios_deploy(devices: &mut HashMap<String, IosDevice>) -> Result<()> {
    let list = ::std::process::Command::new("ios-deploy")
        .stderr(std::process::Stdio::inherit())
        .args(&["-c", "--json", "-t", "1"])
        .output();
    let list = match list {
        Ok(l) => l,
        Err(e) => {
            info!(
                "Could not execute ios-deploy to look for legacy (before iOS 17) iOS devices ({}). Consider installing ios-deploy (`brew install ios-deploy`...) for legacy iOS support.", e);
            return Ok(());
        }
    };
    if !list.status.success() {
        info!(
            "ios-deploy returned an error while listing devices. It this is not expected, you need to make sure `ios-deploy --json -c -t 1` works as expected. ios-deploy is needed for pre-ios17 devices."
            );
        return Ok(());
    }
    // ios-deploy outputs each device as a multiline json dict, with separator or delimiter. make
    // it a json array.
    let list = String::from_utf8(list.stdout)?.replace("}{", "},{");
    let list = format!("[{}]", list);
    let list = ::json::parse(&list)
        .with_context(|| "Could not parse output for: `ios-deploy --json -c -t 1` as json. Please try to make this command work and retry.")?;
    for json in list.members() {
        let device = &json["Device"];
        let id = device["DeviceIdentifier"]
            .as_str()
            .context("DeviceIdentifier expected to be a string")?
            .to_owned();
        let name = device["DeviceName"]
            .as_str()
            .context("DeviceName expected to be a string")?
            .to_owned();
        let arch_cpu = device["modelArch"].as_str().unwrap_or("arm64");
        let ios_version = device["ProductVersion"]
            .as_str()
            .context("ProductVersion expected to be a string")?
            .to_string();
        devices.insert(
            name.clone(),
            IosDevice::new(name, id, &arch_cpu, ios_version)?,
        );
    }
    Ok(())
}
