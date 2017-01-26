extern crate cargo;
#[macro_use]
extern crate clap;
extern crate dinghy;
extern crate env_logger;
#[macro_use]
extern crate log;

use std::{env, path, thread, time};

use cargo::util::important_paths::find_root_manifest_for_wd;

use dinghy::errors::*;

fn main() {
    env_logger::init().unwrap();

    let filtered_env = ::std::env::args()
        .enumerate()
        .filter(|&(ix, ref s)| !(ix == 1 && s == "dinghy"))
        .map(|(_, s)| s);

    let matches = {
            ::clap::App::new("dinghy")
                .arg(::clap::Arg::with_name("DEVICE")
                    .long("device")
                    .takes_value(true)
                    .help("device hint"))
                .subcommand(::clap::SubCommand::with_name("devices"))
                .subcommand(::clap::SubCommand::with_name("test")
                    .arg(::clap::Arg::with_name("TARGET")
                        .long("target")
                        .takes_value(true)
                        .help("target triple (rust conventions)"))
                    .arg(::clap::Arg::with_name("VERBOSE")
                        .short("v")
                        .long("verbose")
                        .multiple(true)
                        .help("Use verbose output"))
                    .arg(::clap::Arg::with_name("LIB").long("lib").help("only the library"))
                    .arg(::clap::Arg::with_name("BIN")
                        .long("bin")
                        .takes_value(true)
                        .help("only the specified binary"))
                    .arg(::clap::Arg::with_name("EXAMPLE")
                        .long("example")
                        .takes_value(true)
                        .help("only the specified example"))
                    .arg(::clap::Arg::with_name("TEST")
                        .long("test")
                        .takes_value(true)
                        .help("only the specified integration test target"))
                    .arg(::clap::Arg::with_name("BENCH")
                        .long("bench")
                        .takes_value(true)
                        .help("only the specified benchmark target"))
                    .arg(::clap::Arg::with_name("RELEASE")
                        .long("release")
                        .help("Build artifacts in release mode, with optimizations"))
                    .arg(::clap::Arg::with_name("FEATURES")
                        .long("features")
                        .takes_value(true)
                        .help("Space-separated list of features to also build"))
                    .arg(::clap::Arg::with_name("ALL_FEATURES")
                        .long("all-features")
                        .help("Build all available features"))
                    .arg(::clap::Arg::with_name("NO_DEFAULT_FEATURES")
                        .long("no-default-features")
                        .help("Do not build the `default` feature"))
                    .arg(::clap::Arg::with_name("ARGS").multiple(true).help("test arguments")))
                .subcommand(::clap::SubCommand::with_name("run")
                    .arg(::clap::Arg::with_name("TARGET")
                        .long("target")
                        .takes_value(true)
                        .help("target triple (rust conventions)"))
                    .arg(::clap::Arg::with_name("VERBOSE")
                        .short("v")
                        .long("verbose")
                        .multiple(true)
                        .help("Use verbose output"))
                    .arg(::clap::Arg::with_name("BIN")
                        .long("bin")
                        .takes_value(true)
                        .help("only the specified binary"))
                    .arg(::clap::Arg::with_name("EXAMPLE")
                        .long("example")
                        .takes_value(true)
                        .help("only the specified example"))
                    .arg(::clap::Arg::with_name("RELEASE")
                        .long("release")
                        .help("Build artifacts in release mode, with optimizations"))
                    .arg(::clap::Arg::with_name("FEATURES")
                        .long("features")
                        .takes_value(true)
                        .help("Space-separated list of features to also build"))
                    .arg(::clap::Arg::with_name("ALL_FEATURES")
                        .long("all-features")
                        .help("Build all available features"))
                    .arg(::clap::Arg::with_name("NO_DEFAULT_FEATURES")
                        .long("no")
                        .short("default")
                        .short("features")
                        .help("Do not build the `default` feature"))
                    .arg(::clap::Arg::with_name("ARGS").multiple(true).help("test arguments")))
                .subcommand(::clap::SubCommand::with_name("bench")
                    .arg(::clap::Arg::with_name("TARGET")
                        .long("target")
                        .takes_value(true)
                        .help("target triple (rust conventions)"))
                    .arg(::clap::Arg::with_name("VERBOSE")
                        .short("v")
                        .long("verbose")
                        .multiple(true)
                        .help("Use verbose output"))
                    .arg(::clap::Arg::with_name("LIB").long("lib").help("only the library"))
                    .arg(::clap::Arg::with_name("BIN")
                        .long("bin")
                        .takes_value(true)
                        .help("only the specified binary"))
                    .arg(::clap::Arg::with_name("EXAMPLE")
                        .long("example")
                        .takes_value(true)
                        .help("only the specified example"))
                    .arg(::clap::Arg::with_name("TEST")
                        .long("test")
                        .takes_value(true)
                        .help("only the specified integration test target"))
                    .arg(::clap::Arg::with_name("BENCH")
                        .long("bench")
                        .takes_value(true)
                        .help("only the specified benchmark target"))
                    .arg(::clap::Arg::with_name("FEATURES")
                        .long("features")
                        .takes_value(true)
                        .help("Space-separated list of features to also build"))
                    .arg(::clap::Arg::with_name("ALL_FEATURES")
                        .long("all-features")
                        .help("Build all available features"))
                    .arg(::clap::Arg::with_name("NO_DEFAULT_FEATURES")
                        .long("no")
                        .short("default")
                        .short("features")
                        .help("Do not build the `default` feature"))
                    .arg(::clap::Arg::with_name("ARGS").multiple(true).help("test arguments")))
                .subcommand(::clap::SubCommand::with_name("build")
                    .arg(::clap::Arg::with_name("TARGET")
                        .long("target")
                        .takes_value(true)
                        .help("target triple (rust conventions)"))
                    .arg(::clap::Arg::with_name("VERBOSE")
                        .short("v")
                        .long("verbose")
                        .multiple(true)
                        .help("Use verbose output"))
                    .arg(::clap::Arg::with_name("BIN")
                        .long("bin")
                        .takes_value(true)
                        .help("only the specified binary"))
                    .arg(::clap::Arg::with_name("EXAMPLE")
                        .long("example")
                        .takes_value(true)
                        .help("only the specified example"))
                    .arg(::clap::Arg::with_name("TEST")
                        .long("test")
                        .takes_value(true)
                        .help("only the specified integration test target"))
                    .arg(::clap::Arg::with_name("BENCH")
                        .long("bench")
                        .takes_value(true)
                        .help("only the specified benchmark target"))
                    .arg(::clap::Arg::with_name("FEATURES")
                        .long("features")
                        .takes_value(true)
                        .help("Space-separated list of features to also build"))
                    .arg(::clap::Arg::with_name("ALL_FEATURES")
                        .long("all-features")
                        .help("Build all available features"))
                    .arg(::clap::Arg::with_name("NO_DEFAULT_FEATURES")
                        .long("no")
                        .short("default")
                        .short("features")
                        .help("Do not build the `default` feature"))
                    .arg(::clap::Arg::with_name("ARGS").multiple(true).help("test arguments")))
                .subcommand(::clap::SubCommand::with_name("lldbproxy"))
        }
        .get_matches_from(filtered_env);

    if let Err(e) = run(matches) {
        println!("{}", e);
        std::process::exit(1);
    }
}

fn run(matches: clap::ArgMatches) -> Result<()> {
    let dinghy = dinghy::Dinghy::probe()?;
    thread::sleep(time::Duration::from_millis(100));
    let mut devices = dinghy.devices()?
        .into_iter()
        .filter(|d| match matches.value_of("DEVICE") {
            Some(filter) => format!("{:?}", d).to_lowercase().contains(&filter.to_lowercase()),
            None => true,
        })
        .collect::<Vec<_>>();
    if devices.len() == 0 {
        Err("No devices found")?
    }
    let d: Box<dinghy::Device> = devices.remove(0);
    match matches.subcommand() {
        ("devices", Some(_matches)) => {
            let devices = dinghy.devices()?;
            for d in devices {
                println!("{:?}", d);
            }
            Ok(())
        }
        ("run", Some(matches)) => prepare_and_run(&*d, "run", matches),
        ("test", Some(matches)) => prepare_and_run(&*d, "test", matches),
        ("bench", Some(matches)) => prepare_and_run(&*d, "bench", matches),
        ("build", Some(matches)) => prepare_and_run(&*d, "build", matches),
        ("lldbproxy", Some(_matches)) => {
            let lldb = d.start_remote_lldb()?;
            println!("lldb running at: {}", lldb);
            loop {
                thread::sleep(time::Duration::from_millis(100));
            }
        }
        (sub, _) => Err(format!("Unknown subcommand {}", sub))?,
    }
}

fn prepare_and_run(d: &dinghy::Device, subcommand: &str, matches: &clap::ArgMatches) -> Result<()> {
    let target = matches.value_of("TARGET").map(|s| s.into()).unwrap_or(d.target());
    if !d.can_run(&*target) {
        Err(format!("device {:?} can not run target {}", d, target))?;
    }
    let runnable = prepare_runnable(&*target, subcommand, matches)?;
    let args = matches.values_of("ARGS").map(|vs| vs.map(|s| s.to_string()).collect()).unwrap_or(vec!());
    for t in runnable {
        let app = d.make_app(&t.1)?;
        if subcommand != "build" {
            d.install_app(&app.as_ref())?;
            d.run_app(app.as_ref(), &*args.iter().map(|s| &s[..]).collect::<Vec<_>>())?;
        }
    }
    Ok(())
}

fn prepare_runnable(target: &str,
                    subcommand: &str,
                    matches: &clap::ArgMatches)
                    -> Result<Vec<(String, path::PathBuf)>> {
    let wd_path = find_root_manifest_for_wd(None, &env::current_dir()?)?;
    let cfg = cargo::util::config::Config::default()?;
    let mode = match subcommand {
        "test" => cargo::ops::CompileMode::Test,
        "bench" => cargo::ops::CompileMode::Bench,
        _ => cargo::ops::CompileMode::Build,
    };
    let features: Vec<String> =
        matches.value_of("FEATURES").unwrap_or("").split(" ").map(|s| s.into()).collect();
    dinghy::build::ensure_shim(&*target)?;
    cfg.configure(matches.occurrences_of("VERBOSE") as u32, None, &None, false, false)?;
    let wd = cargo::core::Workspace::new(&wd_path, &cfg)?;
    let bins = matches.values_of("BIN").map(|vs| vs.map(|s| s.to_string()).collect()).unwrap_or(vec!());
    let tests = matches.values_of("TEST").map(|vs| vs.map(|s| s.to_string()).collect()).unwrap_or(vec!());
    let examples = matches.values_of("EXAMPLE").map(|vs| vs.map(|s| s.to_string()).collect()).unwrap_or(vec!());
    let benches = matches.values_of("BENCH").map(|vs| vs.map(|s| s.to_string()).collect()).unwrap_or(vec!());
    let filter = cargo::ops::CompileFilter::new(matches.is_present("LIB"), &bins, &tests, &examples, &benches);
    let options = cargo::ops::CompileOptions {
        config: &cfg,
        jobs: None,
        target: Some(&*target),
        features: &*features,
        all_features: matches.is_present("ALL_FEATURES"),
        no_default_features: matches.is_present("NO_DEFAULT_FEATURES"),
        spec: &[],
        filter: filter,
        release: subcommand == "bench" || matches.is_present("RELEASE"),
        mode: mode,
        message_format: cargo::ops::MessageFormat::Human,
        target_rustdoc_args: None,
        target_rustc_args: None,
    };
    let compilation = cargo::ops::compile(&wd, &options)?;
    if subcommand == "run" {
        Ok(compilation.binaries.iter().take(1).map(|t| ("main".into(), t.clone())).collect::<Vec<_>>())
    } else {
        Ok(compilation.tests.iter().map(|t| (t.1.clone(), t.2.clone())).collect::<Vec<_>>())
    }
}
