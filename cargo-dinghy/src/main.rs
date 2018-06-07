#[macro_use]
extern crate clap;
extern crate dinghy_lib;
extern crate error_chain;
extern crate itertools;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use clap::ArgMatches;
use dinghy_lib::{ Build, BuildArgs, RunEnv };
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
use std::ffi::{ OsStr, OsString };
use std::sync::Arc;
use std::thread;
use std::time;
use ErrorKind;

/*
#[derive(Debug)]
pub struct Matches<'a> {
    pub dinghy: ArgMatches<'a>,
    pub raw_subcommand: Vec<&'a OsStr>,
    pub cargo: ArgMatches<'a>,
}

impl<'a> Matches<'a> {
    pub fn parse<I>(args: I) -> Matches<'a>
        where I: IntoIterator<Item=&'a OsStr> {
        let filtered_args = args.into_iter()
            .enumerate()
            .filter(|&(ix, ref s)| !(ix == 1 && *s == "dinghy"))
            .map(|pair| pair.1);
        let (dinghy, raw_subcommand) = CargoDinghyCli::parse_dinghy_args(filtered_args);
        if raw_subcommand.len() == 0 {
            panic!("expected subcommand, none found")
        }
        let mut cargo_args:Vec<&OsStr> = vec!(raw_subcommand[0]);
        cargo_args.extend(raw_subcommand.iter());
        let cargo = CargoDinghyCli::parse_subcommands(cargo_args.into_iter());
        Matches {
            dinghy,
            raw_subcommand,
            cargo,
        }
    }
}
*/

fn init_logger(matches: &ArgMatches) {
    if env::var("RUST_LOG").is_err() {
        let dinghy_verbosity = match matches.occurrences_of("VERBOSE") as isize - matches.occurrences_of("QUIET") as isize {
            -2 => "error",
            -1 => "warn",
            0 => "info",
            1 => "debug",
            _ => "trace",
        };
        env::set_var("RUST_LOG", format!("cargo_dinghy={},dinghy={}", dinghy_verbosity, dinghy_verbosity));
    };
    pretty_env_logger::init();
}

fn main() {
    let owned_args:Vec<OsString> = env::args_os().collect();
    let mut args:Vec<&OsStr> = owned_args.iter().map(|s| s.as_os_str()).collect();
    if args[0] == "dinghy" {
        args.remove(0);
    }
    let result = if args[0] == "runner" {
        runner(&args)
    } else {
        cargo(&args)
    };

    if let Err(e) = result {
        match e.kind() {
            &ErrorKind::PackagesCannotBeCompiledForPlatform(_) => {
                error!("{}", e.display_chain());
                std::process::exit(3)
            }
            /*
            &ErrorKind::Cargo(ref cargo) => {
                error!("Cargo error: {}", cargo.to_string().split("\n").next().unwrap_or(""));
                println!("{}", cargo);
                std::process::exit(1);
            },
            */
            _ => {
                error!("{:?}", e.display_chain());
                std::process::exit(1);
            }
        };
    }
}

fn declare_common_args<'a, 'b>(app: clap::App<'a, 'b>) -> clap::App<'a, 'b> {
    use clap::Arg;
    app.arg(Arg::with_name("DEVICE")
        .short("d")
        .long("device")
        .takes_value(true)
        .help("device hint"))
    .arg(Arg::with_name("PLATFORM")
        .long("platform")
        .takes_value(true)
        .help("Use a specific platform (build only)"))
    .arg(Arg::with_name("VERBOSE")
        .short("v")
        .long("verbose")
        .multiple(true)
        .help("Raise the level of verbosity"))
    .arg(Arg::with_name("QUIET")
        .short("q")
        .long("quiet")
        .multiple(true)
        .help("Lower the level of verbosity"))
    .arg(Arg::with_name("ARGS")
        .multiple(true)
        .help("subcommand and arguments"))
}

fn cargo(args:&[&OsStr]) -> Result<()> {
    let app = clap::App::new("dinghy runner");
    let app = declare_common_args(app);
    let matches = app.get_matches_from(args);
    init_logger(&matches);

    let conf = Arc::new(dinghy_config(current_dir().unwrap())?);
    let dinghy = Dinghy::probe(&conf)?;
    let project = Project::new(&conf);
    let (platform, device) = select_platform_and_device_from_cli(&matches, &dinghy)?;

    info!("Targeting platform '{}' and device '{}'",
          platform.id(), device.as_ref().map(|it| it.id()).unwrap_or("<none>"));

    let build_args = BuildArgs {
        cargo_args: matches.values_of("ARGS").unwrap().map(|a| a.to_string()).collect::<Vec<_>>(),
        verbose: matches.is_present("VERBOSE"),
        forced_overlays: vec!()
    };
    platform.build(&project, &build_args)?;
    Ok(())
}

fn runner(args:&[&OsStr]) -> Result<()> {
    let app = clap::App::new("dinghy runner");
    let app = declare_common_args(app);
    let matches = app.get_matches_from(args);
    init_logger(&matches);

    let conf = Arc::new(dinghy_config(current_dir().unwrap())?);
    let dinghy = Dinghy::probe(&conf)?;
    let (platform, device) = select_platform_and_device_from_cli(&matches, &dinghy)?;

    let device = device.as_ref().ok_or("No device found")?;
    let args = arg_as_string_vec(&matches, "ARGS");
    let envs = arg_as_string_vec(&matches, "ENVS");

    let exe = &args[0];

    let args = args.iter().skip(1).map(|s| &s[..]).collect::<Vec<_>>();
    let envs = envs.iter().map(|s| &s[..]).collect::<Vec<_>>();
//    let _build_bundles = if sub_args.is_present("DEBUGGER") {
        debug!("Debug app");
//        device.debug_app(&project, runnable, run_env, &*args, &*envs)?
//    } else {
        debug!("Run app");
//        device.run_app(&project, runnable, run_env, &*args, &*envs)?
//    };

    // FIXME
    /*
    if sub_args.is_present("CLEANUP") {
            device.clean_app(&build_bundle)?;
    }
    */
    Ok(())
}

/*
fn run_command(matches: &Matches) -> Result<()> {
    let conf = Arc::new(dinghy_config(current_dir().unwrap())?);
    let compiler = Arc::new(Compiler::from_args(matches.cargo.subcommand().1.unwrap()));
    let dinghy = Dinghy::probe(&conf, &compiler)?;
    let project = Project::new(&conf);
    match matches.cargo.subcommand() {
        ("all-devices", Some(_)) => return show_all_devices(&dinghy),
        ("all-platforms", Some(_)) => return show_all_platforms(&dinghy),
        _ => {}
    };

    let (platform, device) = select_platform_and_device_from_cli(&matches.dinghy, &dinghy)?;

    match matches.cargo.subcommand() {
        ("devices", Some(_)) => show_all_devices_for_platform(&dinghy, platform),
        ("lldbproxy", Some(_)) => run_lldb(device),
        ("runner", Some(sub_matches)) => runner(&device, &project, &sub_matches),
        (sub, _) => Err(format!("Unknown dinghy command '{}'", sub))?,
    }
}
*/

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
    for pf in dinghy.platforms() {
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
