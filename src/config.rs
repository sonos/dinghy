use std::io::Read;
use std::{collections, fs, path };

use errors::*;

#[derive(Debug)]
pub struct TestData {
    pub base: path::PathBuf,
    pub source: String,
    pub target: String,
}

#[derive(Debug, Default)]
pub struct Configuration {
    pub ssh_devices: collections::BTreeMap<String, SshDeviceConfiguration>,
    pub test_data: Vec<TestData>
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct ConfigurationFileContent {
    pub ssh_devices: collections::BTreeMap<String, SshDeviceConfiguration>,
    pub test_data: collections::BTreeMap<String, String>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SshDeviceConfiguration {
    pub hostname: String,
    pub username: String,
    pub target: String,
}

impl Configuration {
    fn merge(&mut self, file:&path::Path, other: ConfigurationFileContent) {
        self.ssh_devices.extend(other.ssh_devices);
        for (target, source) in other.test_data {
            self.test_data.push(TestData {
                base: file.to_path_buf(),
                source: source,
                target: target,
            })
        }
    }
}

fn read_config_file<P: AsRef<path::Path>>(file: P) -> Result<ConfigurationFileContent> {
    let mut data = String::new();
    let mut fd = fs::File::open(file)?;
    fd.read_to_string(&mut data)?;
    let mut parser = ::toml::Parser::new(&*data);
    let value = parser.parse().unwrap();
    let mut decoder = ::toml::Decoder::new(::toml::Value::Table(value));
    Ok(::serde::Deserialize::deserialize(&mut decoder)?)
}

pub fn config<P: AsRef<path::Path>>(dir: P) -> Result<Configuration> {
    let mut conf = Configuration::default();
    let mut files_to_try = vec!();
    let dir = dir.as_ref().to_path_buf();
    let mut d = dir.as_path();
    while d.parent().is_some() {
        files_to_try.push(d.join(".dinghy.toml"));
        d = d.parent().unwrap();
    }
    files_to_try.push(d.join(".dinghy.toml"));
    if let Some(home) = ::std::env::home_dir() {
        files_to_try.push(home.join(".dinghy.html"))
    }
    for file in files_to_try {
        if path::Path::new(&file).exists() {
            info!("Loading configuration from {:?}", file);
            conf.merge(&file, read_config_file(&file)?);
        } else {
            debug!("No configuration found at {:?}", file);
        }
    }
    Ok(conf)
}
