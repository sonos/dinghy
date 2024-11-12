use itertools::Itertools;
use serde::de;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::Read;
use std::result;
use std::{collections, fs, path};

use crate::errors::*;

#[derive(Clone, Debug)]
pub struct TestData {
    pub id: String,
    pub base: path::PathBuf,
    pub source: String,
    pub target: String,
    pub copy_git_ignored: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct TestDataConfiguration {
    pub copy_git_ignored: bool,
    pub source: String,
    pub target: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DetailedTestDataConfiguration {
    pub source: String,
    pub copy_git_ignored: bool,
    pub target: Option<String>,
}

impl<'de> de::Deserialize<'de> for TestDataConfiguration {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct TestDataVisitor;

        impl<'de> de::Visitor<'de> for TestDataVisitor {
            type Value = TestDataConfiguration;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(
                    "a path like \"tests/my_test_data\" or a \
                     detailed dependency like { source = \
                     \"tests/my_test_data\", copy_git_ignored = true }",
                )
            }

            fn visit_str<E>(self, s: &str) -> result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(TestDataConfiguration {
                    copy_git_ignored: false,
                    source: s.to_owned(),
                    target: None,
                })
            }

            fn visit_map<V>(self, map: V) -> result::Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'de>,
            {
                let mvd = de::value::MapAccessDeserializer::new(map);
                let detailed = DetailedTestDataConfiguration::deserialize(mvd)?;
                Ok(TestDataConfiguration {
                    copy_git_ignored: detailed.copy_git_ignored,
                    source: detailed.source,
                    target: detailed.target,
                })
            }
        }

        deserializer.deserialize_any(TestDataVisitor)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Configuration {
    pub platforms: collections::BTreeMap<String, PlatformConfiguration>,
    pub ssh_devices: collections::BTreeMap<String, SshDeviceConfiguration>,
    pub script_devices: collections::BTreeMap<String, ScriptDeviceConfiguration>,
    pub test_data: Vec<TestData>,
    pub skip_source_copy: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
struct ConfigurationFileContent {
    pub platforms: Option<collections::BTreeMap<String, PlatformConfiguration>>,
    pub ssh_devices: Option<collections::BTreeMap<String, SshDeviceConfiguration>>,
    pub script_devices: Option<collections::BTreeMap<String, ScriptDeviceConfiguration>>,
    pub test_data: Option<collections::BTreeMap<String, TestDataConfiguration>>,
    pub skip_source_copy: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct PlatformConfiguration {
    pub deb_multiarch: Option<String>,
    pub env: Option<collections::HashMap<String, String>>,
    pub overlays: Option<collections::HashMap<String, OverlayConfiguration>>,
    pub rustc_triple: Option<String>,
    pub sysroot: Option<String>,
    pub toolchain: Option<String>,
}

impl PlatformConfiguration {
    pub fn empty() -> Self {
        PlatformConfiguration {
            deb_multiarch: None,
            env: None,
            overlays: None,
            rustc_triple: None,
            sysroot: None,
            toolchain: None,
        }
    }

    pub fn env(&self) -> Vec<(String, String)> {
        self.env
            .as_ref()
            .map(|it| {
                it.iter()
                    .map(|(key, value)| (key.to_string(), value.to_string()))
                    .collect_vec()
            })
            .unwrap_or(vec![])
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct OverlayConfiguration {
    pub path: String,
    pub scope: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SshDeviceConfiguration {
    pub hostname: String,
    pub username: String,
    pub port: Option<u16>,
    pub path: Option<String>,
    pub target: Option<String>,
    pub toolchain: Option<String>,
    pub platform: Option<String>,
    #[serde(default)]
    pub remote_shell_vars: collections::HashMap<String, String>,
    pub install_adhoc_rsync_local_path: Option<String>,
    pub use_legacy_scp_protocol_for_adhoc_rsync_copy: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ScriptDeviceConfiguration {
    pub path: String,
    pub platform: Option<String>,
}

impl Configuration {
    pub fn merge(&mut self, file: &path::Path) -> Result<()> {
        let other = read_config_file(&file)?;
        if let Some(pfs) = other.platforms {
            self.platforms.extend(pfs)
        }
        self.ssh_devices
            .extend(other.ssh_devices.unwrap_or(collections::BTreeMap::new()));
        self.script_devices
            .extend(other.script_devices.unwrap_or(collections::BTreeMap::new()));
        for (id, source) in other.test_data.unwrap_or(collections::BTreeMap::new()) {
            // TODO Remove key
            self.test_data.push(TestData {
                id: id.to_string(),
                base: file.to_path_buf(),
                source: source.source.clone(),
                target: source.target.unwrap_or(source.source.clone()),
                copy_git_ignored: source.copy_git_ignored,
            })
        }
        if let Some(skip_source_copy) = other.skip_source_copy {
            self.skip_source_copy = skip_source_copy
        }
        Ok(())
    }
}

fn read_config_file<P: AsRef<path::Path>>(file: P) -> Result<ConfigurationFileContent> {
    let mut data = String::new();
    let mut fd = fs::File::open(file)?;
    fd.read_to_string(&mut data)?;
    Ok(::toml::from_str(&data)?)
}

pub fn dinghy_config<P: AsRef<path::Path>>(dir: P) -> Result<Configuration> {
    let mut conf = Configuration::default();

    let mut files_to_try = vec![];
    let dir = dir.as_ref().to_path_buf();
    let mut d = dir.as_path();
    while d.parent().is_some() {
        files_to_try.push(d.join("dinghy.toml"));
        files_to_try.push(d.join(".dinghy.toml"));
        files_to_try.push(d.join(".dinghy").join("dinghy.toml"));
        files_to_try.push(d.join(".dinghy").join(".dinghy.toml"));
        d = d.parent().unwrap();
    }
    files_to_try.push(d.join(".dinghy.toml"));
    if let Some(home) = dirs::home_dir() {
        if !dir.starts_with(&home) {
            files_to_try.push(home.join("dinghy.toml"));
            files_to_try.push(home.join(".dinghy.toml"));
            files_to_try.push(home.join(".dinghy").join("dinghy.toml"));
            files_to_try.push(home.join(".dinghy").join(".dinghy.toml"));
        }
    }
    for file in files_to_try {
        if path::Path::new(&file).exists() {
            log::debug!("Loading configuration from {:?}", file);
            conf.merge(&file)?;
        } else {
            log::trace!("No configuration found at {:?}", file);
        }
    }

    log::debug!("Configuration: {:#?}", conf);

    Ok(conf)
}

#[cfg(test)]
mod tests {
    #[test]
    fn load_config_with_str_test_data() {
        let config_file = ::std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join("../../../test-ws/test-app/.dinghy.toml");
        super::read_config_file(config_file).unwrap();
    }
}
