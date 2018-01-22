extern crate cargo;
#[macro_use]
extern crate clap;
extern crate dinghy_lib;
extern crate error_chain;
extern crate itertools;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

mod cli;

use std::env;
use std::thread;
use std::time;

use cargo::ops::CompileMode;
use clap::ArgMatches;
use cli::CargoDinghyCli;
use dinghy_lib::compiler::Compiler;
use dinghy_lib::utils::arg_as_string_vec;
use dinghy_lib::device::host::HostDevice;
use dinghy_lib::errors::*;
use dinghy_lib::platform::host::HostPlatform;
use dinghy_lib::project::Project;
use dinghy_lib::Device;
use dinghy_lib::Dinghy;
use dinghy_lib::Platform;
use error_chain::ChainedError;
use itertools::Itertools;
use std::env::current_dir;
use std::sync::Arc;

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

    if let Err(e) = run_command(matches) {
        error!("{}", e.display_chain());
        println!("{}", e.display_chain());
        std::process::exit(1);
    }
}

fn run_command(args: ArgMatches) -> Result<()> {
    let conf = Arc::new(::dinghy_lib::config::config(current_dir().unwrap())?);
    let dinghy = Dinghy::probe(&conf)?;
    let project = Project::new(&conf);
    let (platform, device) = select_platform_and_device_from_cli(&args, &dinghy)?;

    match args.subcommand() {
        ("all-devices", Some(_)) => show_devices(&dinghy, None),
        ("bench", Some(sub_args)) => prepare_and_run(device, project, platform, "bench", sub_args),
        ("build", Some(sub_args)) => build(platform, sub_args),
        ("devices", Some(_)) => show_devices(&dinghy, Some(platform)),
        ("lldbproxy", Some(_)) => run_lldb(device),
        ("run", Some(sub_args)) => prepare_and_run(device, project, platform, "run", sub_args),
        ("test", Some(sub_args)) => prepare_and_run(device, project, platform, "test", sub_args),
        (sub, _) => Err(format!("Unknown dinghy command '{}'", sub))?,
    }
}

fn build(platform: Arc<Box<Platform>>, sub_args: &ArgMatches) -> Result<()> {
    platform.build(&Compiler::from_args(sub_args), CompileMode::Build).and(Ok(()))
}

fn prepare_and_run(
    device: Option<Arc<Box<Device>>>,
    project: Project,
    platform: Arc<Box<Platform>>,
    subcommand: &str,
    sub_args: &ArgMatches,
) -> Result<()> {
    let device = device.ok_or("No device found")?;
    let mode = match subcommand {
        "test" => CompileMode::Test,
        "bench" => CompileMode::Bench,
        _ => CompileMode::Build,
    };

    let build = platform.build(&Compiler::from_args(sub_args), mode)?;
    let args = arg_as_string_vec(sub_args, "ARGS");
    let envs = arg_as_string_vec(sub_args, "ENVS");
    let no_fail_fast = sub_args.is_present("NO_FAIL_FAST");

    for runnable in &build.runnables {
        let build_bundle = device.install_app(&project, &build, &runnable)?;
        let result = if sub_args.is_present("DEBUGGER") {
            device.debug_app(
                &build_bundle,
                &*args.iter().map(|s| &s[..]).collect::<Vec<_>>(),
                &*envs.iter().map(|s| &s[..]).collect::<Vec<_>>(),
            )
        } else {
            device.run_app(
                &build_bundle,
                &*args.iter().map(|s| &s[..]).collect::<Vec<_>>(),
                &*envs.iter().map(|s| &s[..]).collect::<Vec<_>>(),
            )
        };

        if sub_args.is_present("CLEANUP") { device.clean_app(&build_bundle)?; }
        if !no_fail_fast && result.is_err() { return result; }
    }
    Ok(())
}

fn run_lldb(device: Option<Arc<Box<Device>>>) -> Result<()> {
    let device = device.ok_or("No device found")?;
    let lldb = device.start_remote_lldb()?;
    info!("lldb running at: {}", lldb);
    loop {
        thread::sleep(time::Duration::from_millis(100));
    }
}

fn show_devices(dinghy: &Dinghy, platform: Option<Arc<Box<Platform>>>) -> Result<()> {
    let devices = dinghy.devices().into_iter()
        .filter(|device| platform.as_ref().map_or(true, |it| it.is_compatible_with(&***device)))
        .collect::<Vec<_>>();

    if devices.is_empty() {
        error!("No matching device found");
        println!("No matching device found");
    } else {
        for device in devices { println!("{}", device); }
    }
    Ok(())
}

fn select_platform_and_device_from_cli(matches: &ArgMatches, dinghy: &Dinghy) -> Result<(Arc<Box<Platform>>, Option<Arc<Box<Device>>>)> {
    if let Some(platform_name) = matches.value_of("PLATFORM") {
        let platform = dinghy
            .platform_by_name(platform_name)
            .ok_or(format!("No '{}' platform found", platform_name))?;

        let device = dinghy.devices()
            .into_iter()
            .filter(|device| matches.value_of("DEVICE")
                .map(|filter| format!("{}", device).to_lowercase().contains(&filter.to_lowercase()))
                .unwrap_or(true))
            .filter(|it| platform.is_compatible_with(&**it.as_ref()))
            .next();

        Ok((platform, device))
    } else if let Some(device_filter) = matches.value_of("DEVICE") {
        let filtered_devices = dinghy.devices()
            .into_iter()
            .filter(move |it| format!("{}", it).to_lowercase().contains(&device_filter.to_lowercase()))
            .collect_vec();

        // Would need some ordering here to make sure we select the most relevant platform... or else fail if we have several.
        let platform: Result<Arc<Box<Platform>>> = dinghy.platforms()
            .into_iter()
            .filter(|it| filtered_devices.iter().find(|device| it.is_compatible_with((***device).as_ref())).is_some())
            .next().ok_or("No device found".into());

        Ok((platform?, Some(Arc::new(Box::new(HostDevice::new())))))
    } else {
        Ok((Arc::new(HostPlatform::new()?), Some(Arc::new(Box::new(HostDevice::new())))))
    }
}
