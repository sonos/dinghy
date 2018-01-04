extern crate cargo;
extern crate clap;
extern crate dinghy;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use std::env;
use std::thread;
use std::time;

use cargo::ops::CompileMode;
use dinghy::cli::arg_as_string_vec;
use dinghy::cli::CargoDinghyCli;
use dinghy::config::Configuration;
use dinghy::errors::*;
use dinghy::host::HostPlatform;
use dinghy::regular_platform::RegularPlatform;
use dinghy::Device;
use dinghy::Platform;
use std::env::current_dir;


fn main() {
    let filtered_args = env::args()
        .enumerate()
        .filter(|&(ix, ref s)| !(ix == 1 && s == "dinghy"))
        .map(|(_, s)| s);
    let matches = CargoDinghyCli::parse(filtered_args);

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
    let conf = ::dinghy::config::config(current_dir().unwrap())?;
    let platform = platform_from_cli(&conf, &matches)?;
    let devices = dinghy::Dinghy::probe()?.devices()?;

    match matches.subcommand() {
        ("all-devices", Some(_matches)) => { show_devices(devices, None) }
        ("devices", Some(_matches)) => { show_devices(devices, Some(platform)) }
        ("run", Some(subs)) => prepare_and_run(&matches, &*platform, devices, "run", subs),
        ("test", Some(subs)) => prepare_and_run(&matches, &*platform, devices, "test", subs),
        ("bench", Some(subs)) => prepare_and_run(&matches, &*platform, devices, "bench", subs),
        ("build", Some(sub_args)) => { platform.build(CompileMode::Build, sub_args).and(Ok(())) }
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

fn platform_from_cli(conf: &Configuration, matches: &clap::ArgMatches) -> Result<Box<Platform>> {
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

fn prepare_and_run(
    matches: &clap::ArgMatches,
    platform: &dinghy::Platform,
    devices: Vec<Box<Device>>,
    subcommand: &str,
    sub_args: &clap::ArgMatches,
) -> Result<()> {
    let device = find_main_device_for_platform(platform, devices, &matches)?;
    let mode = match subcommand {
        "test" => CompileMode::Test,
        "bench" => CompileMode::Bench,
        _ => CompileMode::Build,
    };
    debug!("Platform {:?}", platform);

    let runnable_list = platform.build(mode, sub_args)?;
    let args = arg_as_string_vec(sub_args, "ARGS");
    let envs = arg_as_string_vec(sub_args, "ENVS");

    for runnable in runnable_list {
        let app = device.make_app(&runnable.source, &runnable.exe)?;
        device.install_app(&app.as_ref())?;
        if sub_args.is_present("DEBUGGER") {
            println!("DEBUGGER");
            device.debug_app(
                app.as_ref(),
                &*args.iter().map(|s| &s[..]).collect::<Vec<_>>(),
                &*envs.iter().map(|s| &s[..]).collect::<Vec<_>>(),
            )?;
        } else {
            device.run_app(
                app.as_ref(),
                &*args.iter().map(|s| &s[..]).collect::<Vec<_>>(),
                &*envs.iter().map(|s| &s[..]).collect::<Vec<_>>(),
            )?;
        }
        if sub_args.is_present("CLEANUP") {
            device.clean_app(&app.as_ref())?;
        }
    }
    Ok(())
}

fn find_main_device_for_platform(
    platform: &Platform,
    devices: Vec<Box<Device>>,
    matches: &clap::ArgMatches,
) -> Result<Box<dinghy::Device>> {
    find_all_devices_for_platform(platform, devices, matches)?
        .into_iter()
        .next()
        .ok_or("No device found".into())
}

fn find_all_devices_for_platform(
    platform: &Platform,
    devices: Vec<Box<Device>>,
    matches: &clap::ArgMatches,
) -> Result<Vec<Box<dinghy::Device>>> {
    Ok(devices.into_iter()
        .filter(|device| platform.is_compatible_with(device.as_ref()))
        .filter(|device| match matches.value_of("DEVICE") {
            Some(filter) => format!("{}", device)
                .to_lowercase()
                .contains(&filter.to_lowercase()),
            None => true,
        })
        .collect::<Vec<_>>())
}
