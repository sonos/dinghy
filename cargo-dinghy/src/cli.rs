use clap::App;
use clap::Arg;
use clap::ArgGroup;
use clap::ArgMatches;
use clap::SubCommand;
use dinghy_lib::compiler::CompileMode;
use dinghy_lib::BuildArgs;
use std::ffi::OsString;

pub struct CargoDinghyCli {}

fn default_app() -> App<'static> {
    App::new("dinghy")
        .version(crate_version!())
        .device()
        .verbose()
        .quiet()
        .overlay()
        .platform()
        .subcommand(
            SubCommand::with_name("all-devices")
                .about("List all devices that can be used with Dinghy"),
        )
        .subcommand(
            SubCommand::with_name("all-platforms").about("List all platforms known to dinghy"),
        )
        .subcommand(
            SubCommand::with_name("bench")
                .about("Run the benchmarks")
                .lib()
                .bin()
                .example()
                .test()
                .bench()
                .package()
                .all()
                .exclude()
                .job()
                .features()
                .no_default_features()
                .no_run()
                .all_features()
                .common_remote()
                .target()
                .verbose()
                .additional_args()
                .strip()
                .bearded(),
        )
        .subcommand(
            SubCommand::with_name("build")
                .about("Compile the current project")
                .package()
                .all()
                .exclude()
                .job()
                .lib()
                .bin()
                .example()
                .test()
                .bench()
                .debug_or_release()
                .features()
                .all_features()
                .no_default_features()
                .target()
                .verbose()
                .additional_args()
                .strip()
                .bearded(),
        )
        .subcommand(
            SubCommand::with_name("clean")
                .about("Remove artifacts that cargo has generated in the past"),
        )
        .subcommand(
            SubCommand::with_name("devices")
                .about("List devices that can be used with Dinghy for the selected platform"),
        )
        .subcommand(SubCommand::with_name("lldbproxy").about("Debug through lldb"))
        .subcommand(
            SubCommand::with_name("run")
                .about("Build and execute src/main.rs")
                .bin()
                .example()
                .package()
                .job()
                .debug_or_release()
                .features()
                .all_features()
                .no_default_features()
                .target()
                .verbose()
                .common_remote()
                .additional_args()
                .strip()
                .bearded(),
        )
        .subcommand(
            SubCommand::with_name("test")
                .about("Run the tests")
                .lib()
                .bin()
                .example()
                .test()
                .bench()
                .all()
                .package()
                .exclude()
                .job()
                .features()
                .all_features()
                .no_default_features()
                .no_run()
                .debug_or_release()
                .target()
                .verbose()
                .common_remote()
                .additional_args()
                .strip()
                .bearded(),
        )
}

impl CargoDinghyCli {
    pub fn parse<I, T>(args: I) -> ArgMatches
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        default_app().get_matches_from(args)
    }

    pub fn build_args_from(matches: &ArgMatches) -> BuildArgs {
        BuildArgs {
            compile_mode: match matches.subcommand() {
                Some(("bench", _)) => CompileMode::Bench,
                Some(("test", _)) => CompileMode::Test,
                _ => CompileMode::Build,
            },
            forced_overlays: arg_as_string_vec(matches, "OVERLAY"),
            verbose: matches.occurrences_of("VERBOSE") > 0,
        }
    }
}

pub trait CargoDinghyCliExt {
    fn additional_args(self) -> Self;
    fn all(self) -> Self;
    fn all_features(self) -> Self;
    fn bin(self) -> Self;
    fn bench(self) -> Self;
    fn common_remote(self) -> Self;
    fn device(self) -> Self;
    fn example(self) -> Self;
    fn exclude(self) -> Self;
    fn exe(self) -> Self;
    fn features(self) -> Self;
    fn job(self) -> Self;
    fn lib(self) -> Self;
    fn no_default_features(self) -> Self;
    fn no_run(self) -> Self;
    fn overlay(self) -> Self;
    fn package(self) -> Self;
    fn platform(self) -> Self;
    fn debug_or_release(self) -> Self;
    fn strip(self) -> Self;
    fn target(self) -> Self;
    fn test(self) -> Self;
    fn verbose(self) -> Self;
    fn quiet(self) -> Self;
    fn bearded(self) -> Self;
}

impl<'a> CargoDinghyCliExt for App<'a> {
    fn additional_args(self) -> Self {
        self.arg(
            Arg::with_name("ARGS")
                .multiple_occurrences(true)
                .help("test arguments"),
        )
    }

    fn all(self) -> Self {
        self.arg(
            Arg::with_name("ALL")
                .long("all")
                .help("Build all packages in the workspace"),
        )
    }

    fn all_features(self) -> Self {
        self.arg(
            Arg::with_name("ALL_FEATURES")
                .long("all-features")
                .help("Build all available features"),
        )
    }

    fn bench(self) -> Self {
        self.arg(
            Arg::with_name("BENCH")
                .long("bench")
                .takes_value(true)
                .help("only the specified benchmark target"),
        )
    }

    fn bin(self) -> Self {
        self.arg(
            Arg::with_name("BIN")
                .long("bin")
                .takes_value(true)
                .help("only the specified binary"),
        )
    }

    fn common_remote(self) -> Self {
        self.arg(
            Arg::with_name("CLEANUP")
                .long("cleanup")
                .takes_value(false)
                .help("cleanup device after complete"),
        )
        .arg(
            Arg::with_name("DEBUGGER")
                .long("debugger")
                .takes_value(false)
                .help("just start debugger"),
        )
        .arg(
            Arg::with_name("ENVS")
                .long("env")
                .takes_value(true)
                .multiple_values(true)
                .help("Space-separated list of env variables to set e.g. RUST_TRACE=trace"),
        )
    }

    fn device(self) -> Self {
        self.arg(
            Arg::with_name("DEVICE")
                .short('d')
                .long("device")
                .takes_value(true)
                .help("device hint"),
        )
    }

    fn example(self) -> Self {
        self.arg(
            Arg::with_name("EXAMPLE")
                .long("example")
                .takes_value(true)
                .help("only the specified example"),
        )
    }

    fn exclude(self) -> Self {
        self.arg(
            Arg::with_name("EXCLUDE")
                .long("exclude")
                .takes_value(true)
                .number_of_values(1)
                .help("Exclude package to from the build"),
        )
    }

    fn exe(self) -> Self {
        self.arg(
            Arg::with_name("EXE")
                .long("exe")
                .takes_value(true)
                .help("Executable to strip"),
        )
    }

    fn features(self) -> Self {
        self.arg(
            Arg::with_name("FEATURES")
                .long("features")
                .takes_value(true)
                .help("Space-separated list of features to also build"),
        )
    }

    fn job(self) -> Self {
        self.arg(
            Arg::with_name("JOBS")
                .long("jobs")
                .short('j')
                .takes_value(true)
                .help("number of concurrent jobs"),
        )
    }

    fn lib(self) -> Self {
        self.arg(Arg::with_name("LIB").long("lib").help("only the library"))
    }

    fn no_default_features(self) -> Self {
        self.arg(
            Arg::with_name("NO_DEFAULT_FEATURES")
                .long("no-default-features")
                .help("Do not build the `default` feature"),
        )
    }

    fn no_run(self) -> Self {
        self.arg(
            Arg::with_name("NO_RUN")
                .long("no-run")
                .help("Compile, but don't run tests or benches"),
        )
    }

    fn strip(self) -> Self {
        self.arg(
            Arg::with_name("STRIP")
                .long("strip")
                .takes_value(false)
                .help("strip the final executable (will have '-stripped' extension)"),
        )
    }

    fn package(self) -> Self {
        self.arg(
            Arg::with_name("SPEC")
                .short('p')
                .long("package")
                .takes_value(true)
                .number_of_values(1)
                .help("Package to bench, build, run or test"),
        )
    }

    fn overlay(self) -> Self {
        self.arg(
            Arg::with_name("OVERLAY")
                .short('o')
                .long("overlay")
                .takes_value(true)
                .number_of_values(1)
                .help("Force the use of an overlay during project build"),
        )
    }

    fn platform(self) -> Self {
        self.arg(
            Arg::with_name("PLATFORM")
                .long("platform")
                .takes_value(true)
                .help("Use a specific platform (build only)"),
        )
    }

    fn debug_or_release(self) -> Self {
        self.arg(
            Arg::with_name("RELEASE")
                .long("release")
                .help("Build artifacts in release mode, with optimizations"),
        )
        .arg(
            Arg::with_name("DEBUG")
                .long("debug")
                .help("Build artifacts in debug mode, without optimizations"),
        )
        .arg(
            Arg::with_name("PROFILE")
                .long("profile")
                .takes_value(true)
                .help("Build artifacts with the specified profile"),
        )
        .group(
            ArgGroup::with_name("BUILD_TYPE")
                .args(&["DEBUG", "RELEASE", "PROFILE"])
                .multiple(false),
        )
    }

    fn target(self) -> Self {
        self.arg(
            Arg::with_name("TARGET")
                .long("target")
                .takes_value(true)
                .help("target triple (rust conventions)"),
        )
    }

    fn test(self) -> Self {
        self.arg(
            Arg::with_name("TEST")
                .long("test")
                .takes_value(true)
                .help("only the specified integration test target"),
        )
    }

    fn verbose(self) -> Self {
        self.arg(
            Arg::with_name("VERBOSE")
                .short('v')
                .long("verbose")
                .multiple_occurrences(true)
                .help("Raise the level of verbosity"),
        )
    }

    fn quiet(self) -> Self {
        self.arg(
            Arg::with_name("QUIET")
                .short('q')
                .long("quiet")
                .multiple_occurrences(true)
                .help("Lower the level of verbosity"),
        )
    }

    fn bearded(self) -> Self {
        self.arg(
            Arg::with_name("BEARDED")
                .long("bearded")
                .help("Do some naughty stuff"),
        )
    }
}

fn arg_as_string_vec(matches: &ArgMatches, option: &str) -> Vec<String> {
    matches
        .values_of(option)
        .map(|vs| vs.map(|s| s.to_string()).collect())
        .unwrap_or(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_app() {
        default_app().debug_assert();
    }
}
