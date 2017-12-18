extern crate cargo;
#[macro_use]
extern crate clap;
extern crate dinghy;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use std::{env, path, thread, time};

use cargo::util::important_paths::find_root_manifest_for_wd;
use clap::SubCommand;
use dinghy::errors::*;
use dinghy::config::Configuration;
use dinghy::regular_platform::RegularPlatform;

trait SnipsClapExt {
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

fn maybe_device_from_cli(matches: &clap::ArgMatches) -> Result<Option<Box<dinghy::Device>>> {
    let dinghy = dinghy::Dinghy::probe()?;
    thread::sleep(time::Duration::from_millis(100));
    let devices = dinghy
        .devices()?
        .into_iter()
        .filter(|d| match matches.value_of("DEVICE") {
            Some(filter) => format!("{:?}", d)
                .to_lowercase()
                .contains(&filter.to_lowercase()),
            None => true,
        })
        .collect::<Vec<_>>();
    let device = devices.into_iter().next();
    if let Some(device) = device {
        info!("Picked device: {}", device.name());
        Ok(Some(device))
    } else {
        info!("No device found");
        Ok(None)
    }
}

fn main() {
    let filtered_env = ::std::env::args()
        .enumerate()
        .filter(|&(ix, ref s)| !(ix == 1 && s == "dinghy"))
        .map(|(_, s)| s);

    let matches = {
        ::clap::App::new("dinghy")
            .version(crate_version!())
            .device()
            .verbose()
            .platform()

            .subcommand(SubCommand::with_name("bench")
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
                .all_features()
                .common_remote()
                .target()
                .verbose()
                .additional_args())

            .subcommand(SubCommand::with_name("build")
                .package()
                .all()
                .exclude()
                .job()
                .lib()
                .bin()
                .example()
                .test()
                .bench()
                .release()
                .features()
                .all_features()
                .no_default_features()
                .target()
                .verbose()
                .additional_args())

            .subcommand(SubCommand::with_name("devices"))

            .subcommand(SubCommand::with_name("lldbproxy"))

            .subcommand(SubCommand::with_name("run")
                .bin()
                .example()
                .package()
                .job()
                .release()
                .features()
                .all_features()
                .no_default_features()
                .target()
                .verbose()
                .common_remote()
                .additional_args())

            .subcommand(SubCommand::with_name("test")
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
                .release()
                .target()
                .verbose()
                .common_remote()
                .additional_args())

    }.get_matches_from(filtered_env);

    if ::std::env::var("RUST_LOG").is_err() {
        let dinghy_verbosity = match matches.occurrences_of("VERBOSE") {
            0 => "warn",
            1 => "info",
            _ => "debug",
        };
        ::std::env::set_var("RUST_LOG", format!("cargo_dinghy={},dinghy={}", dinghy_verbosity, dinghy_verbosity));
    };
    pretty_env_logger::init().unwrap();

    if let Err(e) = run(matches) {
        println!("{}", e);
        std::process::exit(1);
    }
}

fn device_from_cli(
    _conf: &Configuration,
    matches: &clap::ArgMatches,
) -> Result<Box<dinghy::Device>> {
    Ok(maybe_device_from_cli(matches)?.ok_or("No device found")?)
}

fn default_platform_from_cli(
    conf: &Configuration,
    matches: &clap::ArgMatches,
) -> Result<Box<dinghy::Platform>> {
    if let Some(pf) = matches.value_of("PLATFORM") {
        println!("platforms: {:?}", conf.platforms);
        let cf = conf.platforms
            .get(pf)
            .ok_or(format!("platform {} not found in conf", pf))?;
        return RegularPlatform::new(
            pf.to_string(),
            cf.rustc_triple.clone().unwrap(),
            cf.toolchain.clone().unwrap(),
        );
    }
    if let Some(dev) = maybe_device_from_cli(matches)? {
        return dev.platform();
    }
    Err("Could not guess a platform")?
}

fn platform_from_cli(
    conf: &Configuration,
    matches: &clap::ArgMatches,
) -> Result<Box<dinghy::Platform>> {
    default_platform_from_cli(conf, matches)
}

fn run(matches: clap::ArgMatches) -> Result<()> {
    let conf = ::dinghy::config::config(::std::env::current_dir().unwrap())?;

    match matches.subcommand() {
        ("devices", Some(_matches)) => {
            let dinghy = dinghy::Dinghy::probe()?;
            thread::sleep(time::Duration::from_millis(100));
            let devices = dinghy.devices()?;
            for d in devices {
                println!("{:?}", d);
            }
            Ok(())
        }
        ("run", Some(subs)) => prepare_and_run(&conf, &matches, "run", subs),
        ("test", Some(subs)) => prepare_and_run(&conf, &matches, "test", subs),
        ("bench", Some(subs)) => prepare_and_run(&conf, &matches, "bench", subs),
        ("build", Some(subs)) => {
            let pf = platform_from_cli(&conf, &matches)?;
            build(&*pf, cargo::ops::CompileMode::Build, subs)?;
            Ok(())
        }
        ("lldbproxy", Some(_matches)) => {
            let lldb = device_from_cli(&conf, &matches)?.start_remote_lldb()?;
            println!("lldb running at: {}", lldb);
            loop {
                thread::sleep(time::Duration::from_millis(100));
            }
        }
        (sub, _) => Err(format!("Unknown subcommand {}", sub))?,
    }
}

#[derive(Debug)]
struct Runnable {
    name: String,
    exe: path::PathBuf,
    source: path::PathBuf,
}

fn prepare_and_run(
    conf: &Configuration,
    matches: &clap::ArgMatches,
    subcommand: &str,
    sub: &clap::ArgMatches,
) -> Result<()> {
    let d = device_from_cli(&conf, &matches)?;
    let mode = match subcommand {
        "test" => cargo::ops::CompileMode::Test,
        "bench" => cargo::ops::CompileMode::Bench,
        _ => cargo::ops::CompileMode::Build,
    };
    let pf = platform_from_cli(conf, matches)?;
    debug!("Platform {:?}", pf);
    let runnable = build(&*pf, mode, sub)?;
    let args = sub.values_of("ARGS")
        .map(|vs| vs.map(|s| s.to_string()).collect())
        .unwrap_or(vec![]);
    let envs = sub.values_of("ENVS")
        .map(|vs| vs.map(|s| s.to_string()).collect())
        .unwrap_or(vec![]);
    for t in runnable {
        let app = d.make_app(&t.source, &t.exe)?;
        if subcommand != "build" {
            d.install_app(&app.as_ref())?;
            if sub.is_present("DEBUGGER") {
                println!("DEBUGGER");
                d.debug_app(
                    app.as_ref(),
                    &*args.iter().map(|s| &s[..]).collect::<Vec<_>>(),
                    &*envs.iter().map(|s| &s[..]).collect::<Vec<_>>(),
                )?;
            } else {
                d.run_app(
                    app.as_ref(),
                    &*args.iter().map(|s| &s[..]).collect::<Vec<_>>(),
                    &*envs.iter().map(|s| &s[..]).collect::<Vec<_>>(),
                )?;
            }
            if sub.is_present("CLEANUP") {
                d.clean_app(&app.as_ref())?;
            }
        }
    }
    Ok(())
}


fn build(
    pf: &dinghy::Platform,
    mode: cargo::ops::CompileMode,
    matches: &clap::ArgMatches,
) -> Result<Vec<Runnable>> {
    info!("Building for platform {:?}", pf);
    let wd_path = find_root_manifest_for_wd(None, &env::current_dir()?)?;
    let cfg = cargo::util::config::Config::default()?;
    let features: Vec<String> = matches
        .value_of("FEATURES")
        .unwrap_or("")
        .split(" ")
        .map(|s| s.into())
        .collect();
    pf.setup_env()?;
    cfg.configure(
        matches.occurrences_of("VERBOSE") as u32,
        None,
        &None,
        false,
        false,
        &[],
    )?;
    let wd = cargo::core::Workspace::new(&wd_path, &cfg)?;
    let bins = matches
        .values_of("BIN")
        .map(|vs| vs.map(|s| s.to_string()).collect())
        .unwrap_or(vec![]);
    let tests = matches
        .values_of("TEST")
        .map(|vs| vs.map(|s| s.to_string()).collect())
        .unwrap_or(vec![]);
    let examples = matches
        .values_of("EXAMPLE")
        .map(|vs| vs.map(|s| s.to_string()).collect())
        .unwrap_or(vec![]);
    let benches = matches
        .values_of("BENCH")
        .map(|vs| vs.map(|s| s.to_string()).collect())
        .unwrap_or(vec![]);
    let jobs = matches
        .value_of("JOBS")
        .map(|v| v.parse().unwrap());
    let filter = cargo::ops::CompileFilter::new(
        matches.is_present("LIB"),
        &bins,
        false,
        &tests,
        false,
        &examples,
        false,
        &benches,
        false,
        false,
    );
    let excludes = matches
        .values_of("EXCLUDE")
        .map(|vs| vs.map(|s| s.to_string()).collect())
        .unwrap_or(vec![]);
    let packages = matches
        .values_of("SPEC")
        .map(|vs| vs.map(|s| s.to_string()).collect())
        .unwrap_or(vec![]);
    let spec = cargo::ops::Packages::from_flags(
        wd.is_virtual(),
        matches.is_present("ALL"),
        &excludes,
        &packages,
    )?;

    let triple = pf.rustc_triple()?;
    debug!("rustc target triple: {}", triple);
    let options = cargo::ops::CompileOptions {
        config: &cfg,
        jobs,
        target: Some(&triple),
        features: &*features,
        all_features: matches.is_present("ALL_FEATURES"),
        no_default_features: matches.is_present("NO_DEFAULT_FEATURES"),
        spec: spec,
        filter: filter,
        release: mode == cargo::ops::CompileMode::Bench || matches.is_present("RELEASE"),
        mode: mode,
        message_format: cargo::ops::MessageFormat::Human,
        target_rustdoc_args: None,
        target_rustc_args: None,
    };
    let compilation = cargo::ops::compile(&wd, &options)?;
    if mode == cargo::ops::CompileMode::Build {
        Ok(compilation
            .binaries
            .into_iter()
            .take(1)
            .map(|t| {
                Runnable {
                    name: "main".into(),
                    exe: t,
                    source: path::PathBuf::from("."),
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
}
