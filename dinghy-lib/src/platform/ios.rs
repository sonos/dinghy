use compiler::Compiler;
use errors::*;
use std::fmt::Display;
use std::process;
use std::sync::Arc;
use toolchain::Toolchain;
use Build;
use BuildArgs;
use Device;
use Platform;

pub struct IosPlatform {
    pub sim: bool,
    pub toolchain: Toolchain,
    compiler: Arc<Compiler>,
}

impl IosPlatform {
    pub fn new(rustc_triple: &str, compiler:&Arc<Compiler>) -> Result<Box<Platform>> {
        Ok(Box::new(IosPlatform {
            sim: false,
            toolchain: Toolchain {
                rustc_triple: rustc_triple.to_string()
            },
            compiler: Arc::clone(compiler)
        }))
    }

    fn linker_command(&self) -> Result<String> {
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
    fn build(&self, build_args: BuildArgs) -> Result<Build> {
        self.toolchain.setup_cc(self.id().as_str(), "gcc")?;
        self.toolchain.setup_linker(self.id().as_str(),
                                    format!("cc -isysroot {}",
                                            self.linker_command()?.as_str()).as_str())?;

        self.compiler.build(self.rustc_triple(), build_args)
    }

    fn id(&self) -> String {
        self.toolchain.rustc_triple.to_string()
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

