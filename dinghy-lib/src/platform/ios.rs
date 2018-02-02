use compiler::Compiler;
use config::PlatformConfiguration;
use dinghy_helper::build_env::set_env;
use errors::*;
use overlay::Overlayer;
use overlay::overlay_work_dir;
use project::Project;
use std::fmt::Display;
use std::process;
use std::sync::Arc;
use toolchain::Toolchain;
use Build;
use BuildArgs;
use Device;
use Platform;


pub struct IosPlatform {
    id: String,
    pub sim: bool,
    pub toolchain: Toolchain,
    pub configuration: PlatformConfiguration,
    compiler: Arc<Compiler>,
}

impl IosPlatform {
    pub fn new(id: String, rustc_triple: &str, compiler: &Arc<Compiler>, configuration: &PlatformConfiguration) -> Result<Box<Platform>> {
        Ok(Box::new(IosPlatform {
            id,
            sim: false,
            toolchain: Toolchain {
                rustc_triple: rustc_triple.to_string()
            },
            compiler: Arc::clone(compiler),
            configuration: configuration.clone(),
        }))
    }

    fn sysroot_path(&self) -> Result<String> {
        let sdk_name = if self.sim {
            "iphonesimulator"
        } else {
            "iphoneos"
        };
        let xcrun = process::Command::new("xcrun")
            .args(&["--sdk", sdk_name, "--show-sdk-path"])
            .output()?;
        Ok(String::from_utf8(xcrun.stdout)?.trim_right().to_string())
    }
}

impl Platform for IosPlatform {
    fn build(&self, project: &Project, build_args: BuildArgs) -> Result<Build> {
        let sysroot = self.sysroot_path()?;
        Overlayer::overlay(&self.configuration, self, project, &self.toolchain.sysroot)?;
        self.toolchain.setup_cc(self.id().as_str(), "gcc")?;
        set_env("TARGET_SYSROOT", &sysroot);
        self.toolchain.setup_linker(&self.id(),
                                    &format!("cc -isysroot {}", sysroot))?;

        self.compiler.build(self.rustc_triple(), build_args)
    }

    fn id(&self) -> String {
        self.id.to_string()
    }

    fn is_compatible_with(&self, device: &Device) -> bool {
        device.is_compatible_with_ios_platform(self)
    }

    fn rustc_triple(&self) -> Option<&str> {
        Some(&self.toolchain.rustc_triple)
    }
}

impl Display for IosPlatform {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        if self.sim {
            write!(f, "XCode targetting Ios Simulator")
        } else {
            write!(f, "XCode targetting Ios Device")
        }
    }
}

