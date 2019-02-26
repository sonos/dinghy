extern crate cargo;
#[macro_use]
extern crate clap;
extern crate dinghy_lib;
extern crate error_chain;
extern crate itertools;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use clap::ArgMatches;
use cli::CargoDinghyCli;
use dinghy_lib::Build;
use dinghy_lib::compiler::Compiler;
use dinghy_lib::config::dinghy_config;
use dinghy_lib::Device;
use dinghy_lib::Dinghy;
use dinghy_lib::errors::*;
use dinghy_lib::Platform;
use dinghy_lib::project::Project;
use dinghy_lib::utils::arg_as_string_vec;
use error_chain::ChainedError;
use itertools::Itertools;
use std::env;
use std::env::current_dir;
use std::sync::Arc;
use std::thread;
use std::time;
use ErrorKind;

mod cli;

fn main() {
    let filtered_args = env::args()
        .enumerate()
        .filter(|&(ix, ref s)| !(ix == 1 && s == "dinghy"))
        .map(|(_, s)| s);
    let matches = CargoDinghyCli::parse(filtered_args);

    if env::var("RUST_LOG").is_err() {
        let dinghy_verbosity = match matches.occurrences_of("VERBOSE") - matches.occurrences_of("QUIET") {
            0 => "info",
            1 => "debug",
            _ => "trace",
        };
        env::set_var("RUST_LOG", format!("cargo_dinghy={},dinghy={}", dinghy_verbosity, dinghy_verbosity));
    };
    pretty_env_logger::init();

    if let Err(e) = run_command(&matches) {
        match e.kind() {
            &ErrorKind::PackagesCannotBeCompiledForPlatform(_) => {
                error!("{}", e.display_chain());
                std::process::exit(3)
            }
            &ErrorKind::Cargo(ref cargo) => {
                error!("Cargo error: {}", cargo.to_string().split("\n").next().unwrap_or(""));
                println!("{}", cargo);
                std::process::exit(1);
            },
            _ => {
                error!("{}", e.display_chain());
                std::process::exit(1);
            }
        };
    }
}

fn run_command(args: &ArgMatches) -> Result<()> {
    let conf = Arc::new(dinghy_config(current_dir().unwrap())?);
    let compiler = Arc::new(Compiler::from_args(args.subcommand().1.unwrap_or(args)));
    let dinghy = Dinghy::probe(&conf, &compiler)?;
    let project = Project::new(&conf);
    match args.subcommand() {
        ("all-devices", Some(_)) => return show_all_devices(&dinghy),
        ("all-platforms", Some(_)) => return show_all_platforms(&dinghy),
        _ => {}
    };

    let (platform, device) = select_platform_and_device_from_cli(&args, &dinghy)?;
    info!("Targeting platform '{}' and device '{}'",
          platform.id(), device.as_ref().map(|it| it.id()).unwrap_or("<none>"));

    match args.subcommand() {
        ("bench", Some(sub_args)) => prepare_and_run(device, project, platform, args, sub_args),
        ("build", Some(sub_args)) => build(&platform, &project, args, sub_args).and(Ok(())),
        ("clean", Some(_)) => compiler.clean(None),
        ("devices", Some(_)) => show_all_devices_for_platform(&dinghy, platform),
        ("lldbproxy", Some(_)) => run_lldb(device),
        ("run", Some(sub_args)) => prepare_and_run(device, project, platform, args, sub_args),
        ("test", Some(sub_args)) => prepare_and_run(device, project, platform, args, sub_args),
        (sub, _) => Err(format!("Unknown dinghy command '{}'", sub))?,
    }
}

fn build(platform: &Arc<Box<Platform>>,
         project: &Project,
         args: &ArgMatches,
         sub_args: &ArgMatches) -> Result<Build> {
    let build_args = CargoDinghyCli::build_args_from(args);
    let build = platform.build(&project, &build_args)?;

    if sub_args.is_present("STRIP") {
        platform.strip(&build)?;
    }
    Ok(build)
}

fn prepare_and_run(
    device: Option<Arc<Box<Device>>>,
    project: Project,
    platform: Arc<Box<Platform>>,
    args: &ArgMatches,
    sub_args: &ArgMatches,
) -> Result<()> {
    debug!("Build for {}", platform);
    let build = build(&platform.clone(), &project, args, sub_args)?;

    if sub_args.is_present("NO_RUN") {
        return Ok(())
    }

    debug!("Run on {:?}", device);
    let device = device.ok_or("No device found")?;
    let args = arg_as_string_vec(sub_args, "ARGS");
    let envs = arg_as_string_vec(sub_args, "ENVS");

    let args = args.iter().map(|s| &s[..]).collect::<Vec<_>>();
    let envs = envs.iter().map(|s| &s[..]).collect::<Vec<_>>();
    let build_bundles = if sub_args.is_present("DEBUGGER") {
        debug!("Debug app");
        vec![device.debug_app(&project, &build, &*args, &*envs)?]
    } else {
        debug!("Run app");
        device.run_app(&project, &build, &*args, &*envs)?
    };

    if sub_args.is_present("CLEANUP") {
        for build_bundle in build_bundles {
            device.clean_app(&build_bundle)?;
        }
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

fn show_all_devices(dinghy: &Dinghy) -> Result<()> {
    println!("List of available devices for all platforms:");
    show_devices(&dinghy, None)
}

fn show_all_platforms(dinghy: &Dinghy) -> Result<()> {
    let mut platforms = dinghy.platforms();
    platforms.sort_by(|str1, str2| str1.id().cmp(&str2.id()));
    for pf in platforms.iter() {
        println!("* {} {}", pf.id(), pf.rustc_triple().map(|s| format!("({})", s)).unwrap_or("".to_string()));
    }
    Ok(())
}

fn show_all_devices_for_platform(dinghy: &Dinghy, platform: Arc<Box<Platform>>) -> Result<()> {
    println!("List of available devices for platform '{}':", platform.id());
    show_devices(&dinghy, Some(platform))
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

fn select_platform_and_device_from_cli(matches: &ArgMatches,
                                       dinghy: &Dinghy) -> Result<(Arc<Box<Platform>>, Option<Arc<Box<Device>>>)> {
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
        let devices = dinghy.devices()
            .into_iter()
            .filter(move |it| format!("{:?}", it).to_lowercase().contains(&device_filter.to_lowercase()))
            .collect_vec();
        if devices.len() == 0 {
            Err(format!("No devices found for name hint `{}'", device_filter))?;
        }
        devices.into_iter().filter_map(|d| {
            let pf = dinghy.platforms().iter().find(|pf| pf.is_compatible_with(&**d)).cloned();
            debug!("Looking for platform for {}: found {:?}", d.id(), pf.as_ref().map(|p| p.id()));
            pf.map(|it| (it,Some(d)))
        })
        .next()
        .ok_or(format!("No device and platform combination found for device hint `{}'", device_filter).into())
    } else {
        Ok((dinghy.host_platform(), Some(dinghy.host_device())))
    }
}
