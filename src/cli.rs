use clap;

pub trait SnipsClapExt {
    fn additional_args(self) -> Self;
    fn all(self) -> Self;
    fn all_features(self) -> Self;
    fn bin(self) -> Self;
    fn bench(self) -> Self;
    fn common_remote(self) -> Self;
    fn device(self) -> Self;
    fn example(self) -> Self;
    fn exclude(self) -> Self;
    fn features(self) -> Self;
    fn job(self) -> Self;
    fn lib(self) -> Self;
    fn no_default_features(self) -> Self;
    fn package(self) -> Self;
    fn platform(self) -> Self;
    fn release(self) -> Self;
    fn target(self) -> Self;
    fn test(self) -> Self;
    fn verbose(self) -> Self;
}

impl<'a, 'b> SnipsClapExt for clap::App<'a, 'b> {
    fn additional_args(self) -> Self {
        self.arg(::clap::Arg::with_name("ARGS")
            .multiple(true)
            .help("test arguments"))
    }

    fn all(self) -> Self {
        self.arg(::clap::Arg::with_name("ALL")
            .long("all")
            .help("Build all packages in the workspace"))
    }

    fn all_features(self) -> Self {
        self.arg(::clap::Arg::with_name("ALL_FEATURES")
            .long("all-features")
            .help("Build all available features"))
    }

    fn bench(self) -> Self {
        self.arg(::clap::Arg::with_name("BENCH")
            .long("bench")
            .takes_value(true)
            .help("only the specified benchmark target"))
    }

    fn bin(self) -> Self {
        self.arg(::clap::Arg::with_name("BIN")
            .long("bin")
            .takes_value(true)
            .help("only the specified binary"))
    }

    fn common_remote(self) -> Self {
        self
            .arg(::clap::Arg::with_name("CLEANUP")
                .long("cleanup")
                .takes_value(false)
                .help("cleanup device after complete"))
            .arg(::clap::Arg::with_name("DEBUGGER")
                .long("debugger")
                .takes_value(false)
                .help("just start debugger"))
            .arg(::clap::Arg::with_name("ENVS")
                .long("env")
                .takes_value(true)
                .multiple(true)
                .help("Space-separated list of env variables to set e.g. RUST_TRACE=trace"))
    }

    fn device(self) -> Self {
        self.arg(::clap::Arg::with_name("DEVICE")
            .short("d")
            .long("device")
            .takes_value(true)
            .help("device hint"))
    }

    fn example(self) -> Self {
        self.arg(::clap::Arg::with_name("EXAMPLE")
            .long("example")
            .takes_value(true)
            .help("only the specified example"))
    }

    fn exclude(self) -> Self {
        self.arg(::clap::Arg::with_name("EXCLUDE")
            .long("exclude")
            .takes_value(true)
            .multiple(true)
            .number_of_values(1)
            .help("Exclude package to from the build"))
    }

    fn features(self) -> Self {
        self.arg(::clap::Arg::with_name("FEATURES")
            .long("features")
            .takes_value(true)
            .help("Space-separated list of features to also build"))
    }

    fn job(self) -> Self {
        self.arg(::clap::Arg::with_name("JOBS")
            .long("jobs")
            .short("j")
            .takes_value(true)
            .help("number of concurrent jobs"))
    }

    fn lib(self) -> Self {
        self.arg(::clap::Arg::with_name("LIB")
            .long("lib")
            .help("only the library"))
    }

    fn no_default_features(self) -> Self {
        self.arg(::clap::Arg::with_name("NO_DEFAULT_FEATURES")
            .long("no-default-features")
            .help("Do not build the `default` feature"))
    }

    fn package(self) -> Self {
        self.arg(::clap::Arg::with_name("SPEC")
            .short("p")
            .long("package")
            .takes_value(true)
            .multiple(true)
            .number_of_values(1)
            .help("Package to bench, build, run or test"))
    }

    fn platform(self) -> Self {
        self.arg(::clap::Arg::with_name("PLATFORM")
            .long("platform")
            .takes_value(true)
            .help("Use a specific platform (build only)"))
    }

    fn release(self) -> Self {
        self.arg(::clap::Arg::with_name("RELEASE")
            .long("release")
            .help("Build artifacts in release mode, with optimizations"))
    }

    fn target(self) -> Self {
        self.arg(::clap::Arg::with_name("TARGET")
            .long("target")
            .takes_value(true)
            .help("target triple (rust conventions)"))
    }

    fn test(self) -> Self {
        self.arg(::clap::Arg::with_name("TEST")
            .long("test")
            .takes_value(true)
            .help("only the specified integration test target"))
    }

    fn verbose(self) -> Self {
        self.arg(::clap::Arg::with_name("VERBOSE")
            .short("v")
            .long("verbose")
            .multiple(true)
            .help("Sets the level of verbosity"))
    }
}
