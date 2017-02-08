use std::io::Read;
use std::{collections, env, fs, path };

use errors::*;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Configuration {
    pub ssh_devices: collections::BTreeMap<String, SshDeviceConfiguration>,
    pub test_data: collections::BTreeMap<String, String>,
}

impl Configuration {
    fn merge(&mut self, other: Configuration) {
        self.ssh_devices.extend(other.ssh_devices);
        self.test_data.extend(other.test_data);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SshDeviceConfiguration {
    pub hostname: String,
    pub username: String,
    pub target: String,
}

fn read_config_file<P: AsRef<path::Path>>(file: P) -> Result<Configuration> {
    let mut data = String::new();
    let mut fd = fs::File::open(file)?;
    fd.read_to_string(&mut data)?;
    let mut parser = ::toml::Parser::new(&*data);
    let value = parser.parse().unwrap();
    let mut decoder = ::toml::Decoder::new(::toml::Value::Table(value));
    Ok(::serde::Deserialize::deserialize(&mut decoder)?)
}

pub fn config() -> Result<Configuration> {
    let mut conf = Configuration::default();
    let dir = env::current_dir()?;
    let mut files_to_try = vec!();
    let mut d = dir.as_path();
    while d.parent().is_some() {
        files_to_try.push(d.join(".dinghy.toml"));
        d = d.parent().unwrap();
    }
    for file in files_to_try {
        if path::Path::new(&file).exists() {
            info!("Loading configuration from {:?}", file);
            conf.merge(read_config_file(file)?);
        } else {
            debug!("No configuration found at {:?}", file);
        }
    }
    Ok(conf)
}
