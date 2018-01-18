pub use cargo::ops::CompileMode;
use cargo::util::important_paths::find_root_manifest_for_wd;
use cargo::core::Workspace;
use cargo::ops::CompileFilter;
use cargo::ops::CompileOptions;
use cargo::ops::MessageFormat;
use cargo::util::config::Config as CompileConfig;
use cargo::ops as CargoOps;
use cargo::ops::Packages as CompilePackages;
use clap::ArgMatches;
use cli::arg_as_string_vec;
use std::env::current_dir;
use std::path::PathBuf;
use Result;
use Runnable;

pub struct CargoFacade {
    build_command: Box<Fn(CompileMode, Option<&str>) -> Result<Vec<Runnable>>>,
}

impl CargoFacade {
    pub fn from_args(matches: &ArgMatches) -> CargoFacade {
        CargoFacade {
            build_command: CargoFacade::create_build_command(matches),
        }
    }

    fn create_build_command(matches: &ArgMatches) -> Box<Fn(CompileMode, Option<&str>) -> Result<Vec<Runnable>>> {
        let all = matches.is_present("ALL");
        let all_features = matches.is_present("ALL_FEATURES");
        let benches = arg_as_string_vec(matches, "BENCH");
        let bins = arg_as_string_vec(matches, "BIN");
        let features: Vec<String> = matches
            .value_of("FEATURES")
            .unwrap_or("")
            .split(" ")
            .map(|s| s.into())
            .collect();
        let examples = arg_as_string_vec(matches, "EXAMPLE");
        let excludes = arg_as_string_vec(matches, "EXCLUDE");
        let jobs = matches
            .value_of("JOBS")
            .map(|v| v.parse::<u32>().unwrap());
        let lib_only = matches.is_present("LIB");
        let no_default_features = matches.is_present("NO_DEFAULT_FEATURES");
        let packages = arg_as_string_vec(matches, "SPEC");
        let release = matches.is_present("RELEASE");
        let verbosity = matches.occurrences_of("VERBOSE") as u32;
        let tests = arg_as_string_vec(matches, "TEST");

        Box::new(move |compile_mode: CompileMode, rustc_triple: Option<&str>| {
            let release = compile_mode == CompileMode::Bench || release;
            let config = &CompileConfig::default()?;
            config.configure(
                verbosity,
                None,
                &None,
                false,
                false,
                &[],
            )?;
            let wd = Workspace::new(&find_root_manifest_for_wd(None, &current_dir()?)?,
                                    config)?;

            let options = CompileOptions {
                config,
                jobs,
                target: rustc_triple,
                features: &*features,
                all_features,
                no_default_features,
                spec: CompilePackages::from_flags(
                    wd.is_virtual(),
                    all,
                    &excludes,
                    &packages,
                )?,
                filter: CompileFilter::new(
                    lib_only,
                    &bins, false,
                    &tests, false,
                    &examples, false,
                    &benches, false,
                    false, // all_targets
                ),
                release,
                mode: compile_mode,
                message_format: MessageFormat::Human,
                target_rustdoc_args: None,
                target_rustc_args: None,
            };

            let compilation = CargoOps::compile(&wd, &options)?;

            if compile_mode == CompileMode::Build {
                Ok(compilation
                    .binaries
                    .into_iter()
                    .take(1)
                    .map(|t| {
                        Runnable {
                            name: "main".into(),
                            exe: t,
                            source: PathBuf::from("."),
                        }
                    })
                    .collect::<Vec<_>>())
            } else {
                Ok(compilation
                    .tests
                    .into_iter()
                    .map(|(pkg, _, name, exe)| {
                        Runnable {
                            name: name,
                            source: pkg.root().to_path_buf(),
                            exe: exe,
                        }
                    })
                    .collect::<Vec<_>>())
            }
        })
    }

    pub fn build(&self, compile_mode: CompileMode, rustc_triple: Option<&str>) -> Result<Vec<Runnable>> {
        (self.build_command)(compile_mode, rustc_triple)
    }

    pub fn project_dir(&self) -> Result<PathBuf> {
        let wd_path = ::cargo::util::important_paths::find_root_manifest_for_wd(None, &current_dir()?)?;
        Ok(wd_path.parent()
            .ok_or(format!("Couldn't read project directory {}.", wd_path.display()))?
            .to_path_buf())
    }

    pub fn target_dir(&self, rustc_triple: &str) -> Result<PathBuf> {
        Ok(self.project_dir()?.join("target").join(rustc_triple))
    }
}
