mod device;
mod platform;

use crate::{
    config::PlatformConfiguration,
    ohos::platform::{OhosArch, OhosPlatform},
    toolchain::ToolchainConfig,
    utils::LogCommandExt,
    Device, Platform, PlatformManager,
};
use anyhow::{anyhow, bail, Context, Result};
use device::OhosDevice;
use log::debug;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub struct OhosManager {
    hdc: PathBuf,
}

impl PlatformManager for OhosManager {
    fn devices(&self) -> Result<Vec<Box<dyn Device>>> {
        let result = Command::new(&self.hdc)
            .arg("list")
            .arg("targets")
            .log_invocation(3)
            .output()
            .context("Run hdc failed.")?;
        let output = String::from_utf8_lossy(&result.stdout).trim().to_string();
        if output == "[Empty]" {
            return Ok(Vec::new());
        }
        let mut devices = Vec::new();
        // Filter out hdc log and new line
        for id in output
            .lines()
            .map(|x| x.trim())
            .filter(|x| !x.starts_with("["))
            .filter(|x| !x.is_empty())
        {
            let device = OhosDevice::from_id(self.hdc.clone(), id.to_string())
                .context("Create OpenHarmony device from id failed.")?;
            debug!("Discovered OpenHarmony device: ({:?})", device);
            devices.push(Box::new(device) as Box<dyn Device>);
        }
        Ok(devices)
    }

    fn platforms(&self) -> Result<Vec<Box<dyn Platform>>> {
        let ndk = ohos_ndk().context("Find ohos ndk path failed.")?;
        let tools = ndk.join("llvm");
        let sysroot = ndk.join("sysroot");
        let bin_dir = tools.join("bin");
        let version = ohos_ndk_version(&ndk).context("Get ndk version failed.")?;
        let ndk_major_version: usize = version
            .split(".")
            .next()
            .and_then(|major| major.parse().ok())
            .ok_or_else(|| anyhow!("Invalid version found for ohos ndk {:?}", &ndk))?;
        debug!(
            "OpenHarmony ndk: {:?}, ndk version: {}, ndk_major_version: {}",
            ndk, version, ndk_major_version
        );
        let mut platforms = vec![];
        for (arch, rustc_cpu, cc_cpu, binutils_cpu, abi, abi_kind) in [
            (
                OhosArch::Aarch64,
                "aarch64",
                "aarch64",
                "aarch64",
                "arm64-v8a",
                "ohos",
            ),
            (
                OhosArch::Armv7,
                "armv7",
                "armv7a",
                "arm",
                "armeabi-v7a",
                "ohos",
            ),
            (
                OhosArch::X86_64,
                "x86_64",
                "x86_64",
                "x86_64",
                "x86_64",
                "ohos",
            ),
        ] {
            let id = format!("auto-ohos-{}", abi);
            let toolchain_config = ToolchainConfig {
                bin_dir: bin_dir.clone(),
                rustc_triple: format!("{}-unknown-linux-{}", rustc_cpu, abi_kind),
                // pkgconfig is placed in `native/llvm/python3/lib/pkgconfig/`
                root: tools.clone(),
                sysroot: Some(sysroot.clone()),
                cc: "clang".to_string(),
                cxx: "clang++".to_string(),
                binutils_prefix: format!("{}-linux-{}", binutils_cpu, abi_kind),
                cc_prefix: format!("{}-linux-{}", cc_cpu, abi_kind),
            };
            platforms.push(
                OhosPlatform::new(
                    PlatformConfiguration::default(),
                    arch,
                    id,
                    toolchain_config,
                    ndk_major_version,
                    ndk.clone(),
                )
                .context("Create ohos platform failed.")?,
            );
        }
        Ok(platforms)
    }
}

impl OhosManager {
    pub fn probe() -> Option<OhosManager> {
        match hdc() {
            Ok(hdc) => {
                debug!("HDC found: {:?}", hdc);
                Some(OhosManager { hdc })
            }
            Err(_) => {
                debug!("hdc not found in path, ohos disabled");
                None
            }
        }
    }
}

fn hdc() -> Result<PathBuf> {
    if let Ok(hdc) = std::env::var("DINGHY_OHOS_HDC") {
        return Ok(hdc.into());
    }
    if let Ok(hdc) = ::which::which("hdc") {
        return Ok(hdc);
    }
    if let Ok(ndk) = std::env::var("OHOS_SDK_HOME") {
        return Ok(Path::new(&ndk).join("default/openharmony/toolchains/hdc"));
    }
    bail!("The hdc couldn't be found")
}

fn ohos_ndk() -> Result<PathBuf> {
    if let Ok(sdk) = std::env::var("OHOS_SDK_HOME") {
        return Ok(Path::new(&sdk).join("default/openharmony/native/"));
    }
    bail!("The ndk couldn't be found")
}

fn ohos_ndk_version(ndk: &Path) -> Result<String> {
    let meta_path = ndk.join("oh-uni-package.json");
    let meta = fs::read_to_string(&meta_path)
        .with_context(|| anyhow!("Read NDK meta file failed: {}", meta_path.display()))?;
    let mut meta = json::parse(&meta)
        .with_context(|| anyhow!("Parse ohos ndk meta file failed: {}", meta_path.display()))?;
    let ndk_version = meta
        .remove("version")
        .as_str()
        .context("No version in oh-uni-package.json file")?
        .to_string();
    Ok(ndk_version)
}
