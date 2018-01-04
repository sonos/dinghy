use cargo::util::important_paths::find_root_manifest_for_wd;
use cargo::core::Workspace;
use cargo::ops::CompileFilter;
use cargo::ops::CompileMode;
use cargo::ops::CompileOptions;
use cargo::ops::MessageFormat;
use cargo::util::config::Config as CompileConfig;
use cargo::ops as CargoOps;
use cargo::ops::Packages as CompilePackages;
use clap::ArgMatches;
use std::env::current_dir;
use std::path::PathBuf;
use Result;
use Runnable;

pub struct CargoFacade/*<'a>*/ {
    //    matches: ArgMatches<'a>,
//    rustc_triple: String,
    build_command: Box<Fn(CompileMode, Option<&str>) -> Result<Vec<Runnable>>>,
}

impl /*<'a>*/ CargoFacade/*<'a>*/ {
    pub fn from_args(matches: &ArgMatches/*<'a>*//*, rustc_triple: String*/) -> CargoFacade/*<'a>*/ {
        CargoFacade {
//            matches: matches.clone(),
//            rustc_triple: rustc_triple,
            build_command: CargoFacade::create_build_command(matches),
        }
    }

//    pub fn build(&self, mode: CompileMode) -> Result<Vec<Runnable>> {
//        let all = self.matches.is_present("ALL");
//        let all_features = self.matches.is_present("ALL_FEATURES");
//        let benches = as_string_vec(&self.matches, "BENCH");
//        let bins = as_string_vec(&self.matches, "BIN");
//        let features: Vec<String> = self.matches
//            .value_of("FEATURES")
//            .unwrap_or("")
//            .split(" ")
//            .map(|s| s.into())
//            .collect();
//        let examples = as_string_vec(&self.matches, "EXAMPLE");
//        let excludes = as_string_vec(&self.matches, "EXCLUDE");
//        let jobs = self.matches
//            .value_of("JOBS")
//            .map(|v| v.parse::<u32>().unwrap());
//        let lib_only = self.matches.is_present("LIB");
//        let no_default_features = self.matches.is_present("NO_DEFAULT_FEATURES");
//        let packages = as_string_vec(&self.matches, "SPEC");
//        let release = mode == CompileMode::Bench || self.matches.is_present("RELEASE");
//        let verbosity = self.matches.occurrences_of("VERBOSE") as u32;
//        let tests = as_string_vec(&self.matches, "TEST");
//
//        let config = &CompileConfig::default()?;
//        config.configure(
//            verbosity,
//            None,
//            &None,
//            false,
//            false,
//            &[],
//        )?;
//        let wd = Workspace::new(&find_root_manifest_for_wd(None, &current_dir()?)?,
//                                config)?;
//
//        debug!("rustc target triple: {}", self.rustc_triple);
//        let options = CompileOptions {
//            config,
//            jobs,
//            target: Some(&self.rustc_triple),
//            features: &*features,
//            all_features,
//            no_default_features,
//            spec: CompilePackages::from_flags(
//                wd.is_virtual(),
//                all,
//                &excludes,
//                &packages,
//            )?,
//            filter: CompileFilter::new(
//                lib_only,
//                &bins, false,
//                &tests, false,
//                &examples, false,
//                &benches, false,
//                false, // all_targets
//            ),
//            release,
//            mode: mode,
//            message_format: MessageFormat::Human,
//            target_rustdoc_args: None,
//            target_rustc_args: None,
//        };
//
//        let compilation = CargoOps::compile(&wd, &options)?;
//
//        if mode == CompileMode::Build {
//            Ok(compilation
//                .binaries
//                .into_iter()
//                .take(1)
//                .map(|t| {
//                    Runnable {
//                        name: "main".into(),
//                        exe: t,
//                        source: PathBuf::from("."),
//                    }
//                })
//                .collect::<Vec<_>>())
//        } else {
//            Ok(compilation
//                .tests
//                .into_iter()
//                .map(|(pkg, _, name, exe)| {
//                    Runnable {
//                        name: name,
//                        source: pkg.root().to_path_buf(),
//                        exe: exe,
//                    }
//                })
//                .collect::<Vec<_>>())
//        }
//    }

//    pub fn build_binaries(&self) -> Result<Vec<Runnable>> {
//        Ok(self.compile(CompileMode::Build)?
//            .binaries
//            .into_iter()
//            .take(1)
//            .map(|t| {
//                Runnable {
//                    name: "main".into(),
//                    exe: t,
//                    source: PathBuf::from("."),
//                }
//            })
//            .collect::<Vec<_>>())
//    }
//
//    pub fn build_test(&self) -> Result<Vec<Runnable>> {
//        Ok(self.compile(CompileMode::Test)?
//            .tests
//            .into_iter()
//            .map(|(pkg, _, name, exe)| {
//                Runnable {
//                    name: name,
//                    source: pkg.root().to_path_buf(),
//                    exe: exe,
//                }
//            })
//            .collect::<Vec<_>>())
//    }
//
//    fn compile(&self, mode: CompileMode) -> CargoResult<Compilation> {
//        let all = self.matches.is_present("ALL");
//        let all_features = self.matches.is_present("ALL_FEATURES");
//        let benches = as_string_vec(&self.matches, "BENCH");
//        let bins = as_string_vec(&self.matches, "BIN");
//        let features: Vec<String> = self.matches
//            .value_of("FEATURES")
//            .unwrap_or("")
//            .split(" ")
//            .map(|s| s.into())
//            .collect();
//        let examples = as_string_vec(&self.matches, "EXAMPLE");
//        let excludes = as_string_vec(&self.matches, "EXCLUDE");
//        let jobs = self.matches
//            .value_of("JOBS")
//            .map(|v| v.parse::<u32>().unwrap());
//        let lib_only = self.matches.is_present("LIB");
//        let no_default_features = self.matches.is_present("NO_DEFAULT_FEATURES");
//        let packages = as_string_vec(&self.matches, "SPEC");
//        let release = mode == CompileMode::Bench || self.matches.is_present("RELEASE");
//        let verbosity = self.matches.occurrences_of("VERBOSE") as u32;
//        let tests = as_string_vec(&self.matches, "TEST");
//
//        let config = &CompileConfig::default()?;
//        config.configure(
//            verbosity,
//            None,
//            &None,
//            false,
//            false,
//            &[],
//        )?;
//        let wd = Workspace::new(&find_root_manifest_for_wd(None, &current_dir()?)?,
//                                config)?;
//
//        debug!("rustc target triple: {}", self.rustc_triple);
//        let options = CompileOptions {
//            config,
//            jobs,
//            target: Some(&self.rustc_triple),
//            features: &*features,
//            all_features,
//            no_default_features,
//            spec: CompilePackages::from_flags(
//                wd.is_virtual(),
//                all,
//                &excludes,
//                &packages,
//            )?,
//            filter: CompileFilter::new(
//                lib_only,
//                &bins, false,
//                &tests, false,
//                &examples, false,
//                &benches, false,
//                false, // all_targets
//            ),
//            release,
//            mode: mode,
//            message_format: MessageFormat::Human,
//            target_rustdoc_args: None,
//            target_rustc_args: None,
//        };
//
//        Ok(CargoOps::compile(&wd, &options)?)
//    }

    fn create_build_command(matches: &ArgMatches) -> Box<Fn(CompileMode, Option<&str>) -> Result<Vec<Runnable>>> {
        let all = matches.is_present("ALL");
        let all_features = matches.is_present("ALL_FEATURES");
        let benches = as_string_vec(matches, "BENCH");
        let bins = as_string_vec(matches, "BIN");
        let features: Vec<String> = matches
            .value_of("FEATURES")
            .unwrap_or("")
            .split(" ")
            .map(|s| s.into())
            .collect();
        let examples = as_string_vec(matches, "EXAMPLE");
        let excludes = as_string_vec(matches, "EXCLUDE");
        let jobs = matches
            .value_of("JOBS")
            .map(|v| v.parse::<u32>().unwrap());
        let lib_only = matches.is_present("LIB");
        let no_default_features = matches.is_present("NO_DEFAULT_FEATURES");
        let packages = as_string_vec(matches, "SPEC");
        let release = matches.is_present("RELEASE");
        let verbosity = matches.occurrences_of("VERBOSE") as u32;
        let tests = as_string_vec(matches, "TEST");

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

    pub fn build(&self, rustc_triple: Option<&str>) -> Result<Vec<Runnable>> {
        (self.build_command)(CompileMode::Build, rustc_triple)
    }
}

fn as_string_vec(matches: &ArgMatches, option: &str) -> Vec<String> {
    matches.values_of(option)
        .map(|vs| vs.map(|s| s.to_string()).collect())
        .unwrap_or(vec![])
}
