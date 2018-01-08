extern crate bindgen;
extern crate gcc;
#[macro_use]
extern crate error_chain;

use std::env;
use std::env::current_dir;
use std::path::PathBuf;

error_chain! {
    foreign_links {
        Io(::std::io::Error);
        EnvVar(::std::env::VarError);
        StringFromUtf8(::std::string::FromUtf8Error);
    }
}

pub struct DinghyHelper {}

fn path_as_str(path: &PathBuf) -> Result<&str> {
    Ok(path.to_str()
        .ok_or(format!("Not a valid UTF-8 path ({})", path.display()))?)
}

impl DinghyHelper {
    pub fn bindgen_with_cross_compilation() -> Result<bindgen::Builder> {
        Ok(bindgen::Builder::default()
            .clang_arg("--verbose")
            .detect_toolchain()?
            .include_gcc_system_headers()?)
    }

    pub fn link_lib(lib_name: &str) -> Result<()> {
        if DinghyHelper::is_cross_compiling()? {
            let lib_dir = DinghyHelper::env_sysroot()?.join("usr").join("lib");
            println!("cargo:rustc-link-search={}", path_as_str(&lib_dir)?);
        }
        println!("cargo:rustc-link-lib={}", lib_name);
        Ok(())
    }

    pub fn is_cross_compiling() -> Result<bool> {
        Ok(env::var("TARGET")? != env::var("HOST")?)
    }

    fn env_sysroot() -> Result<PathBuf> {
        env::var_os("TARGET_SYSROOT").map(PathBuf::from).chain_err(|| "You must either define a TARGET_SYSROOT or use Dinghy to build.")
    }
}

pub trait BindGenBuilderExt {
    fn detect_toolchain(self) -> Result<bindgen::Builder>;

    fn generate_default_binding(self) -> Result<()>;

    fn header_in_current_dir(self, header_file_name: &str) -> Result<bindgen::Builder>;

    fn include_gcc_system_headers(self) -> Result<bindgen::Builder>;
}

impl BindGenBuilderExt for bindgen::Builder {
    fn detect_toolchain(self) -> Result<bindgen::Builder> {
        if DinghyHelper::is_cross_compiling()? {
            let target = env::var("TARGET")?;
            Ok(self
                .clang_arg(format!("--sysroot={}", path_as_str(&DinghyHelper::env_sysroot()?)?))
                .clang_arg(format!("--target={}", target)))
        } else {
            Ok(self)
        }
    }

    fn generate_default_binding(self) -> Result<()> {
        let out_path = env::var("OUT_DIR").map(PathBuf::from)?.join("bindings.rs");
        Ok(self.generate()
            .expect("Unable to generate bindings")
            .write_to_file(out_path)?)
    }

    fn header_in_current_dir(self, header_file_name: &str) -> Result<bindgen::Builder> {
        let header_path = current_dir().map(PathBuf::from)?.join(header_file_name);
        Ok(self.header(header_path.to_str()
            .ok_or(format!("Not a valid UTF-8 path ({})", header_path.display()))?))
    }

    fn include_gcc_system_headers(self) -> Result<bindgen::Builder> {
        if DinghyHelper::is_cross_compiling()? {
            // Add a path to the private headers for the target compiler. Borderline,
            // as we are likely using a gcc header with clang frontend.
            let path = gcc::Build::new()
                .get_compiler()
                .to_command()
                .arg("--print-file-name=include")
                .output()
                .chain_err(|| "Couldn't find target GCC executable.")
                .and_then(|output| if output.status.success() {
                    Ok(String::from_utf8(output.stdout)?)
                } else {
                    bail!("Couldn't determine target GCC include dir.")
                })?;

            Ok(self
                .clang_arg("-isystem")
                .clang_arg(path.trim()))
        } else {
            Ok(self)
        }
    }
}
