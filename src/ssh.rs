use std::{ collections, fs, path };
use errors::*;
use ::{Device, PlatformManager};

use std::io::Read;

#[derive(Serialize, Deserialize, Debug)]
struct Configuration {
    ssh_devices: collections::BTreeMap<String, SshDeviceConfiguration>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SshDeviceConfiguration {
    hostname: String,
    target: String,
}

#[derive(Clone,Debug)]
pub struct SshDevice {
    id: String,
    config: SshDeviceConfiguration
}

impl Device for SshDevice {
    fn name(&self) -> &str {
        &*self.id
    }
    fn id(&self) -> &str {
        &*self.id
    }
    fn target(&self) -> String {
        "armv7-unknown-linux-gnueabihf".to_string()
    }
    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
    }
    fn make_app(&self, app: &path::Path) -> Result<path::PathBuf> {
        Ok(app.into())
    }
    fn install_app(&self, app: &path::Path) -> Result<()> {
        unimplemented!();
    }
    fn run_app(&self, app_path: &path::Path, args: &[&str]) -> Result<()> {
        unimplemented!();
    }
    fn debug_app(&self, _app_path: &path::Path, _args: &[&str]) -> Result<()> {
        unimplemented!()
    }
}

pub struct SshDeviceManager {
}

impl SshDeviceManager {
    pub fn probe() -> Option<SshDeviceManager> {
        Some(SshDeviceManager{})
    }
}

impl PlatformManager for SshDeviceManager {
    fn devices(&self) -> Result<Vec<Box<Device>>> {
        let file = "config.toml";
        let mut data = String::new();
        let mut fd = fs::File::open(file)?;
        fd.read_to_string(&mut data)?;
        let mut parser = ::toml::Parser::new(&*data);
        let value = parser.parse().unwrap();
        let mut decoder = ::toml::Decoder::new(::toml::Value::Table(value));
        let config:Configuration = ::serde::Deserialize::deserialize(&mut decoder)?;
        let devices = config.ssh_devices.into_iter().map(|(k,d)| Box::new(SshDevice { id:k, config: d }) as _).collect();
        Ok(devices)
    }
}

