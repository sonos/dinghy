extern crate cargo;
#[macro_use]
extern crate clap;
extern crate dinghy;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use std::env;
use std::path;
use std::thread;
use std::time;

use cargo::util::important_paths::find_root_manifest_for_wd;
use clap::SubCommand;
use dinghy::cli::SnipsClapExt;
use dinghy::config::Configuration;
use dinghy::errors::*;
use dinghy::host::HostPlatform;
use dinghy::regular_platform::RegularPlatform;
use dinghy::Device;
use dinghy::Platform;


fn main() {
    let filtered_args = env::args()
        .enumerate()
        .filter(|&(ix, ref s)| !(ix == 1 && s == "dinghy"))
        .map(|(_, s)| s);

    let matches = {
        clap::App::new("dinghy")
            .version(crate_version!())
            .device()
            .verbose()
            .platform()

            .subcommand(SubCommand::with_name("all-devices"))

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

    }.get_matches_from(filtered_args);

    if env::var("RUST_LOG").is_err() {
        let dinghy_verbosity = match matches.occurrences_of("VERBOSE") {
            0 => "warn",
            1 => "info",
            _ => "debug",
        };
        env::set_var("RUST_LOG", format!("cargo_dinghy={},dinghy={}", dinghy_verbosity, dinghy_verbosity));
    };
    pretty_env_logger::init().unwrap();

    if let Err(e) = run(matches) {
        println!("{}", e);
        std::process::exit(1);
    }
}

fn run(matches: clap::ArgMatches) -> Result<()> {
    let conf = ::dinghy::config::config(env::current_dir().unwrap())?;
    let platform = platform_from_cli(&conf, &matches)?;
    let devices = dinghy::Dinghy::probe()?.devices()?;

    match matches.subcommand() {
        ("all-devices", Some(_matches)) => { show_devices(devices, None) }
        ("devices", Some(_matches)) => { show_devices(devices, Some(platform)) }
        ("run", Some(subs)) => prepare_and_run(&matches, &*platform, devices, "run", subs),
        ("test", Some(subs)) => prepare_and_run(&matches, &*platform, devices, "test", subs),
        ("bench", Some(subs)) => prepare_and_run(&matches, &*platform, devices, "bench", subs),
        ("build", Some(subs)) => { build(&*platform, cargo::ops::CompileMode::Build, subs).and(Ok(())) }
        ("lldbproxy", Some(_matches)) => {
            let lldb = find_main_device_for_platform(&*platform, devices, &matches)?.start_remote_lldb()?;
            println!("lldb running at: {}", lldb);
            loop {
                thread::sleep(time::Duration::from_millis(100));
            }
        }
        (sub, _) => Err(format!("Unknown subcommand {}", sub))?,
    }
}

fn platform_from_cli(
    conf: &Configuration,
    matches: &clap::ArgMatches,
) -> Result<Box<Platform>> {
    Ok(match matches.value_of("PLATFORM") {
        Some(platform_name) => {
            let cf = conf.platforms
                .get(platform_name)
                .ok_or(format!("platform {} not found in conf", platform_name))?;
            RegularPlatform::new(
                platform_name.to_string(),
                cf.rustc_triple.clone().unwrap(),
                cf.toolchain.clone().unwrap(),
            )
        }
        None => HostPlatform::new(),
    }?)
}

fn show_devices(devices: Vec<Box<Device>>, platform: Option<Box<Platform>>) -> Result<()> {
    let devices = devices.into_iter()
        .filter(|device| platform.as_ref().map_or(true, |it| it.is_compatible_with(device.as_ref())))
        .collect::<Vec<_>>();

    if devices.is_empty() {
        println!("No matching device found");
    } else {
        for device in devices { println!("{}", device); }
    }
    Ok(())
}

#[derive(Debug)]
struct Runnable {
    name: String,
    exe: path::PathBuf,
    source: path::PathBuf,
}

fn prepare_and_run(
    matches: &clap::ArgMatches,
    platform: &dinghy::Platform,
    devices: Vec<Box<Device>>,
    subcommand: &str,
    sub: &clap::ArgMatches,
) -> Result<()> {
    let d = find_main_device_for_platform(platform, devices, &matches)?;
    let mode = match subcommand {
        "test" => cargo::ops::CompileMode::Test,
        "bench" => cargo::ops::CompileMode::Bench,
        _ => cargo::ops::CompileMode::Build,
    };
    debug!("Platform {:?}", platform);
    let runnable = build(&*platform, mode, sub)?;
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
    platform: &dinghy::Platform,
    mode: cargo::ops::CompileMode,
    matches: &clap::ArgMatches,
) -> Result<Vec<Runnable>> {
    info!("Building for platform {:?}", platform);
    let wd_path = find_root_manifest_for_wd(None, &env::current_dir()?)?;
    let cfg = cargo::util::config::Config::default()?;
    let features: Vec<String> = matches
        .value_of("FEATURES")
        .unwrap_or("")
        .split(" ")
        .map(|s| s.into())
        .collect();
    platform.setup_env()?;
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

    let triple = platform.rustc_triple()?;
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
