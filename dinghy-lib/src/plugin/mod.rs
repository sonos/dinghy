use crate::config::{PlatformConfiguration, ScriptDeviceConfiguration, SshDeviceConfiguration};
use crate::platform::regular_platform::RegularPlatform;
use crate::{Configuration, Device, Platform, PlatformManager};
use anyhow::{anyhow, bail, Context, Result};
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::sync::Arc;
use std::{env, fs};

/// This platform manager will auto-detect any executable in the PATH that starts with
/// `cargo-dinghy-` and try to use them as a plugin to provide devices and platforms.
///
/// To be a valid plugin, an executable must implement the following subcommands:
/// - `devices`: must output a TOML file with a `DevicePluginOutput` structure
/// - `platforms`: must output a TOML file with a `BTreeMap<String, PlatformConfiguration>` structure
///
/// Here is example of output for a `cargo-dinghy-foo` plugin configuring a `bar` device and a `baz`
/// platform:
///
/// ```no_compile
/// $ cargo-dinghy-foo devices
/// [ssh_devices.bar]
/// hostname = "127.0.0.1"
/// username = "user"
///
/// $ cargo-dinghy-foo platforms
/// [baz]
/// rustc_triple = "aarch64-unknown-linux-gnu"
/// toolchain = "/path/to/toolchain"
/// ```
/// This is quite useful if you have a bench of devices and platforms that can be auto-detected
/// or are already configured in another tool.
pub struct PluginManager {
    conf: Arc<Configuration>,
    auto_detected_plugins: Vec<String>,
}

impl PluginManager {
    pub fn probe(conf: Arc<Configuration>) -> Option<PluginManager> {
        let auto_detected_plugins = auto_detect_plugins();

        if auto_detected_plugins.is_empty() {
            debug!("No auto-detected plugins found");
            None
        } else {
            debug!("Auto-detected plugins: {:?}", auto_detected_plugins);
            Some(Self {
                conf,
                auto_detected_plugins,
            })
        }
    }
    fn create_script_devices(
        &self,
        provider: &String,
        script_devices: BTreeMap<String, ScriptDeviceConfiguration>,
    ) -> Vec<Box<dyn Device>> {
        script_devices
            .into_iter()
            .filter_map(|(id, conf)| {
                if self.conf.script_devices.get(&id).is_none() {
                    debug!("registering script device {id} from {provider}");
                    Some(Box::new(crate::script::ScriptDevice { id, conf }) as _)
                } else {
                    debug!("ignoring script device {id} from {provider} as is was already registered in configuration");
                    None
                }
            })
            .collect()
    }

    fn create_ssh_devices(
        &self,
        provider: &String,
        ssh_devices: BTreeMap<String, SshDeviceConfiguration>,
    ) -> Vec<Box<dyn Device>> {
        ssh_devices.into_iter().filter_map(|(id, conf)| {
            if self.conf.script_devices.get(&id).is_none() {
                debug!("registering ssh device {id} from {provider}");
                Some(Box::new(crate::ssh::SshDevice {
                    id,
                    conf,
                }) as _)
            } else {
                debug!("ignoring ssh device {id} from {provider} as is was already registered in configuration");
                None
            }
        }).collect()
    }
}

impl PlatformManager for PluginManager {
    fn devices(&self) -> Result<Vec<Box<dyn Device>>> {
        let mut result: Vec<Box<dyn Device>> = vec![];

        self.auto_detected_plugins.iter().for_each(|provider| {
            match get_devices_from_plugin(provider) {
                Ok(DevicePluginOutput{script_devices, ssh_devices}) => {
                    if let Some(script_devices) = script_devices {
                        result.append(&mut self.create_script_devices(provider, script_devices))
                    }

                    if let Some(ssh_devices) = ssh_devices {
                        result.append(&mut self.create_ssh_devices(provider, ssh_devices))
                    }

                }
                Err(e) => {
                    debug!(
                        "failed to get devices from auto detected script provider: {provider}, {e:?}",
                    );
                }
            }
        });

        Ok(result)
    }

    fn platforms(&self) -> anyhow::Result<Vec<Box<dyn Platform>>> {
        let mut script_platforms = BTreeMap::new();

        self.auto_detected_plugins.iter().for_each(
            |provider| match get_platforms_from_plugin(provider) {
                Ok(platforms) => {
                    platforms.into_iter().for_each(|(id, platform)| {
                        if script_platforms.get(&id).is_none() && self.conf.platforms.get(&id).is_none() {
                            debug!("registering platform {id} from {provider}");
                            script_platforms.insert(id.clone(), platform);
                        } else {
                            debug!(
                                "ignoring platform {id} from plugin {provider} as is was already registered"
                            );
                        }
                    });
                }
                Err(e) => {
                    debug!(
                        "failed to get platforms from auto detected script provider: {provider}, {:?}",
                        e
                    );
                }
            },
        );

        Ok(script_platforms.into_values().collect())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DevicePluginOutput {
    pub ssh_devices: Option<BTreeMap<String, SshDeviceConfiguration>>,
    pub script_devices: Option<BTreeMap<String, ScriptDeviceConfiguration>>,
}

fn get_devices_from_plugin(plugin: &str) -> Result<DevicePluginOutput> {
    let output = Command::new(plugin).arg("devices").output()?;

    if !output.status.success() {
        bail!("failed to get devices from auto detected script provider: {:?}, non success return code", plugin);
    }

    Ok(toml::from_str(
        &String::from_utf8(output.stdout)
            .with_context(|| format!("Failed to parse string output from {plugin} devices"))?,
    )
    .with_context(|| format!("Failed to parse toml output from {plugin} devices"))?)
}

fn get_platforms_from_plugin(plugin: &str) -> Result<BTreeMap<String, Box<dyn Platform>>> {
    let output = Command::new(plugin).arg("platforms").output()?;

    if !output.status.success() {
        bail!("failed to get platforms from auto detected script provider: {:?}, non success return code", plugin);
    }

    let platform_configs = toml::from_str::<BTreeMap<String, PlatformConfiguration>>(
        &String::from_utf8(output.stdout)
            .with_context(|| format!("Failed to parse string output from {plugin} platforms"))?,
    )
    .with_context(|| format!("Failed to parse toml output from {plugin} platforms"))?;

    platform_configs
        .into_iter()
        .map(|(name, conf)| {
            let triple = conf
                .rustc_triple
                .clone()
                .ok_or_else(|| anyhow!("Platform {name} from {plugin} has no rustc_triple"))?;
            let toolchain = conf
                .toolchain
                .clone()
                .ok_or_else(|| anyhow!("Toolchain missing for platform {name} from {plugin}"))?;
            Ok((
                name.clone(),
                RegularPlatform::new(conf, name, triple, toolchain)?,
            ))
        })
        .collect()
}

// dinghy will auto-detect any executable in the PATH that starts with `cargo-dinghy-` and try to
// use it as a plugin.
fn auto_detect_plugins() -> Vec<String> {
    let mut binaries = Vec::new();

    if let Some(paths) = env::var_os("PATH") {
        for path in env::split_paths(&paths) {
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if let Some(file_name) = path.file_name().and_then(|name| name.to_str()) {
                        if file_name.starts_with("cargo-dinghy-")
                            && (path.is_file()
                                && path
                                    .metadata()
                                    .map(|m| m.permissions().mode() & 0o111 != 0)
                                    .unwrap_or(false))
                        {
                            binaries.push(file_name.to_string());
                        }
                    }
                }
            }
        }
    }
    binaries.sort(); // ensure a deterministic order
    binaries
}
