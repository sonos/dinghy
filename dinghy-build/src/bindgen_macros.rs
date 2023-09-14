/// Create a new `bindgen::Builder` set up and ready for cross-compilation.
///
/// This macro should be used for bindgen versions from 0.49 and above.
#[macro_export]
macro_rules! dinghy_bindgen {
    () => {{
        let bindgen = $crate::dinghy_bindgen_pre_0_49!();

        if $crate::build::is_cross_compiling().expect("Couldn't determine if it is cross-compiling")
        {
            bindgen.detect_include_paths(false)
        } else {
            bindgen
        }
    }};
}

/// Compatibility macro for bindgen versions below 0.49
#[macro_export]
macro_rules! dinghy_bindgen_pre_0_49 {
    () => {{
        use $crate::build::is_cross_compiling;
        use $crate::build_env::sysroot_path;
        use $crate::utils::path_to_str;
        use $crate::{Result, Context};

        fn apple_patch(builder: bindgen::Builder) -> Result<bindgen::Builder> {
            if is_cross_compiling()? {
                let target = env::var("TARGET")?;
                if target.contains("apple") && target.contains("aarch64") {
                    // The official Apple tools use "-arch arm64" instead of specifying
                    // -target directly; -arch only works when the default target is
                    // Darwin-based to put Clang into "Apple mode" as it were. But it does
                    // sort of explain why arm64 works better than aarch64, which is the
                    // preferred name everywhere else.
                    return Ok(builder
                        .clang_arg(format!("-arch"))
                        .clang_arg(format!("arm64")));
                }
            }
            Ok(builder)
        }

        fn libclang_path_patch(builder: bindgen::Builder) -> Result<bindgen::Builder> {
            if is_cross_compiling()? {
                if let Ok(libclang_path) = env::var("DINGHY_BUILD_LIBCLANG_PATH") {
                    env::set_var("LIBCLANG_PATH", libclang_path)
                }
            }
            Ok(builder)
        }

        fn detect_toolchain(builder: bindgen::Builder) -> Result<bindgen::Builder> {
            if is_cross_compiling()? {
                let target = env::var("TARGET")?;
                let builder = if let Ok(_) = env::var("TARGET_SYSROOT") {
                    builder.clang_arg(format!("--sysroot={}", path_to_str(&sysroot_path()?)?))
                } else {
                    println!("cargo:warning=No Sysroot detected, assuming the target is baremetal. If you have a sysroot, you must either define a TARGET_SYSROOT or use Dinghy to build your project.");
                    builder
                };
                Ok(builder.clang_arg(format!("--target={}", target)))
            } else {
                Ok(builder)
            }
        }

        fn include_gcc_system_headers(builder: bindgen::Builder) -> Result<bindgen::Builder> {
            if is_cross_compiling()? {
                // Add a path to the private headers for the target compiler. Borderline,
                // as we are likely using a gcc header with clang frontend.
                let path = cc::Build::new()
                    .get_compiler()
                    .to_command()
                    .arg("--print-file-name=include")
                    .output()
                    .with_context(|| "Couldn't find target GCC executable.")
                    .and_then(|output| {
                        if output.status.success() {
                            Ok(String::from_utf8(output.stdout)?)
                        } else {
                            panic!("Couldn't determine target GCC include dir.")
                        }
                    })?;

                Ok(builder.clang_arg("-isystem").clang_arg(path.trim()))
            } else {
                Ok(builder)
            }
        }

        libclang_path_patch(
            apple_patch(
                include_gcc_system_headers(
                    detect_toolchain(bindgen::Builder::default().clang_arg("--verbose")).unwrap(),
                )
                .unwrap(),
            )
            .unwrap()
        )
        .unwrap()
    }};
}

/// Generate a file containing the bindgen bindings in a standard path.
///
/// The standard path is `${OUT_DIR}/bindings.rs`.
///
/// To use it, simply perform the call like
/// `generate_default_bindgen_bindings!(bindgen_builder)`
#[macro_export]
macro_rules! generate_bindgen_bindings {
    ($builder:expr) => {{
        let out_path = env::var("OUT_DIR")
            .map(PathBuf::from)
            .expect("Couldn't convert OUT_DIR var into a path")
            .join("bindings.rs");
        $builder
            .generate()
            .expect("Unable to generate bindings")
            .write_to_file(out_path)
            .expect("Unable to write the bindings in the file")
    }};
}
