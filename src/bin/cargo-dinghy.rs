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

    let matches = clap_app!(dinghy =>
        (@arg DEVICE: --device +takes_value "device hint")
        (@subcommand devices =>)
        (@subcommand test =>
            (@arg TARGET: --target +takes_value "target triple (rust conventions)")
            (@arg LIB: --lib "only the library")
            (@arg BIN: --bin +takes_value "only the specified binary")
            (@arg EXAMPLE: --example +takes_value "only the specified example")
            (@arg TEST: --test +takes_value "only the specified integration test target")
            (@arg BENCH: --bench +takes_value "only the specified benchmark target")
            (@arg RELEASE: --release "Build artifacts in release mode, with optimizations")
            (@arg FEATURES: --features +takes_value "Space-separated list of features to also build")
            (@arg ALL_FEATURES: --all_features  "Build all available features")
            (@arg NO_DEFAULT_FEATURES: --no-default-features "Do not build the `default` feature")
            (@arg ARGS: +multiple "test arguments")
        )
        (@subcommand run =>
            (@arg TARGET: --target +takes_value "target triple (rust conventions)")
            (@arg BIN: --bin +takes_value "only the specified binary")
            (@arg EXAMPLE: --example +takes_value "only the specified example")
            (@arg RELEASE: --release "Build artifacts in release mode, with optimizations")
            (@arg FEATURES: --features +takes_value "Space-separated list of features to also build")
            (@arg ALL_FEATURES: --all_features  "Build all available features")
            (@arg NO_DEFAULT_FEATURES: --no-default-features "Do not build the `default` feature")
            (@arg ARGS: +multiple "test arguments")
        )
        (@subcommand bench =>
            (@arg TARGET: --target +takes_value "target triple (rust conventions)")
            (@arg LIB: --lib "only the library")
            (@arg BIN: --bin +takes_value "only the specified binary")
            (@arg EXAMPLE: --example +takes_value "only the specified example")
            (@arg TEST: --test +takes_value "only the specified integration test target")
            (@arg BENCH: --bench +takes_value "only the specified benchmark target")
            (@arg FEATURES: --features +takes_value "Space-separated list of features to also build")
            (@arg ALL_FEATURES: --all_features  "Build all available features")
            (@arg NO_DEFAULT_FEATURES: --no-default-features "Do not build the `default` feature")
            (@arg ARGS: +multiple "test arguments")
        )
        (@subcommand build =>
            (@arg TARGET: --target +takes_value "target triple (rust conventions)")
            (@arg BIN: --bin +takes_value "only the specified binary")
            (@arg EXAMPLE: --example +takes_value "only the specified example")
            (@arg TEST: --test +takes_value "only the specified integration test target")
            (@arg BENCH: --bench +takes_value "only the specified benchmark target")
            (@arg FEATURES: --features +takes_value "Space-separated list of features to also build")
            (@arg ALL_FEATURES: --all_features  "Build all available features")
            (@arg NO_DEFAULT_FEATURES: --no-default-features "Do not build the `default` feature")
            (@arg ARGS: +multiple "test arguments")
        )
        (@subcommand lldbproxy =>
        )
    )
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
    let runnable = prepare_runnable(d, subcommand, matches)?;
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

fn prepare_runnable(device: &dinghy::Device,
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
    let target = matches.value_of("TARGET").map(|s| s.into()).unwrap_or(device.target());
    dinghy::build::ensure_shim(&*target)?;
    cfg.configure(0, None, &None, false, false)?;
    let wd = cargo::core::Workspace::new(&wd_path, &cfg)?;
    let bins = matches.values_of("BIN").map(|vs| vs.map(|s| s.to_string()).collect()).unwrap_or(vec!());
    let tests = matches.values_of("TEST").map(|vs| vs.map(|s| s.to_string()).collect()).unwrap_or(vec!());
    let examples = matches.values_of("EXAMPLE").map(|vs| vs.map(|s| s.to_string()).collect()).unwrap_or(vec!());
    let benches = matches.values_of("BENCHES").map(|vs| vs.map(|s| s.to_string()).collect()).unwrap_or(vec!());
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
