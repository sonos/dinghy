use compiler::Compiler;
use compiler::CompileMode;
use errors::*;
use std::fmt::Display;
use std::process;
use toolchain::Toolchain;
use Build;
use Device;
use Platform;

#[derive(Debug)]
pub struct IosPlatform {
    pub sim: bool,
    pub toolchain: Toolchain,
}

impl IosPlatform {
    pub fn new(rustc_triple: &str) -> Result<Box<Platform>> {
        Ok(Box::new(IosPlatform {
            sim: false,
            toolchain: Toolchain {
                rustc_triple: rustc_triple.to_string()
            },
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
    fn build(&self, compiler: &Compiler, compile_mode: CompileMode) -> Result<Build> {
        self.toolchain.setup_cc(self.id().as_str(), "gcc")?;
        self.toolchain.setup_linker(self.id().as_str(),
                                    format!("cc -isysroot {}",
                                            self.linker_command()?.as_str()).as_str())?;

        compiler.build(self.rustc_triple(), compile_mode)
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

