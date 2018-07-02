extern crate clap;
extern crate dinghy_lib;
extern crate error_chain;
extern crate itertools;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

use clap::ArgMatches;
use dinghy_lib::BuildArgs;
use dinghy_lib::config::dinghy_config;
use dinghy_lib::Device;
use dinghy_lib::Dinghy;
use dinghy_lib::errors::*;
use dinghy_lib::Platform;
use dinghy_lib::project::Project;
use dinghy_lib::utils::arg_as_string_vec;
use error_chain::ChainedError;
use itertools::Itertools;
use std::{ env, path, thread, time };
use std::env::current_dir;
use std::ffi::{ OsStr, OsString };
use std::sync::Arc;
use ErrorKind;

fn main() {
    let owned_args:Vec<OsString> = env::args_os().collect();
    let mut args:Vec<&OsStr> = owned_args.iter().map(|s| s.as_os_str()).collect();
    if args[1] == "dinghy" {
        args.remove(1);
    }
    let result = match args[1].to_str() {
        Some("runner") => runner(&args),
        Some("all-devices") => show_all_devices(&args),
        Some("all-platforms") => show_all_platforms(&args),
        _ => cargo(&args),
    };
    if let Err(e) = result {
        match e.kind() {
            &ErrorKind::PackagesCannotBeCompiledForPlatform(_) => {
                error!("{}", e.display_chain());
                std::process::exit(3)
            }
            &ErrorKind::Child(ref cargo) => std::process::exit(*cargo as i32),
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
    .arg(Arg::with_name("ALL")
        .long("all")
        .help("Build/test/bench all packages, filtered by Cargo.toml dinghy metadata"))
    .arg(Arg::with_name("ENVS")
        .long("env")
        .takes_value(true)
        .multiple(true)
        .number_of_values(1)
        .help("env variable to set e.g. RUST_TRACE=trace"))
}

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

#[derive(Debug)]
struct DinghyCtxt {
    dinghy: dinghy_lib::Dinghy,
    conf: Arc<dinghy_lib::config::Configuration>,
    project: Project,
    platform: Arc<Box<Platform>>,
    device: Option<Arc<Box<Device>>>,
    verbose: bool,
    args: Vec<String>,
    envs: Vec<String>,
    all: bool,
}

impl DinghyCtxt {
    pub fn new(args: &[&OsStr]) -> Result<DinghyCtxt> {
        // try the longest argument string that we can match
        let cargo_sub_len = (0..args.len()).filter_map(|i| {
            let args = args[0..args.len()-i].to_vec();
            let app = clap::App::new("cargo dinghy");
            let mut app = declare_common_args(app);
            match app.get_matches_from_safe_borrow(&args) {
                Ok(_) => Some(i),
                Err(_) => None,
            }
        }).next();
        let app = clap::App::new("cargo dinghy");
        let app = declare_common_args(app);
        let (matches, dinghy_arg_len) = if let Some(i) = cargo_sub_len {
            let dinghy_arg_len = args.len() - i;
            (app.get_matches_from(&args[0..dinghy_arg_len]), dinghy_arg_len)
        } else {
            app.get_matches_from(args);
            unreachable!()
        };
        init_logger(&matches);

        let conf = Arc::new(dinghy_config(current_dir().unwrap())?);
        let dinghy = Dinghy::probe(&conf)?;
        let project = Project::new(&conf);
        let (platform, device) = select_platform_and_device_from_cli(&matches, &dinghy)?;
        Ok(DinghyCtxt {
            verbose: matches.is_present("VERBOSE"),
            conf,
            dinghy,
            project,
            platform,
            device,
            args: args[dinghy_arg_len..].iter().map(|s| s.to_str().expect("could not convert arg to string (utf-8 ?)").to_string()).collect(),
            envs: arg_as_string_vec(&matches, "ENVS"),
            all: matches.is_present("ALL"),
        })
    }
}

fn cargo(args:&[&OsStr]) -> Result<()> {
    use std::io::Write;
    let ctx = DinghyCtxt::new(args)?;

    info!("Targeting platform '{}' and device '{}'",
          ctx.platform.id(), ctx.device.as_ref().map(|it| it.id()).unwrap_or("<none>"));

    { // scope file, it must be closed before the build is started
        let target = ::dinghy_lib::utils::project_root()?.join("target");
        std::fs::create_dir_all(&target)?;
        let run_env_file = target.join("cargo-dinghy-run-env");
        let mut run_env_file = std::fs::File::create(run_env_file)?;
        for env in ctx.envs {
            write!(run_env_file, "{}", env)?;
        }
    }

    let build_args = BuildArgs {
        cargo_args: ctx.args,
        all: ctx.all,
        verbose: ctx.verbose,
        forced_overlays: vec!(),
        device: ctx.device
    };
    ctx.platform.build(&ctx.project, &build_args)?;
    Ok(())
}

fn runner(args:&[&OsStr]) -> Result<()> {
    use std::io::BufRead;
    let ctx = DinghyCtxt::new(&args[1..])?;
    let device = ctx.device.ok_or("No device found")?;

    info!("Targeting platform '{}' and device '{:?}'",
          ctx.platform.id(), device);

    let run_env_file = ::dinghy_lib::utils::project_root()?.join("target").join("cargo-dinghy-run-env");
    let run_env_file = std::fs::File::open(run_env_file)?;
    let run_env_file = std::io::BufReader::new(run_env_file);
    let mut envs = vec!();
    for line in run_env_file.lines() {
        envs.push(line?);
    }

    let double_dash = args.iter().position(|a| a.to_str() == Some("--")).ok_or("Could not find -- in command line")?;
    let exe = path::PathBuf::from(args[double_dash+1]);
    let args:Vec<_> = args[double_dash+2..].iter().map(|s| s.to_str().expect("could not convert arg to string (utf-8 ?)").to_string()).collect();
    info!("Runner {:?} on {}", exe, device);
    let artefacts = dinghy_lib::cargo::restore_artefacts_metadata()?;
    let artefact = artefacts.into_iter()
        .find(|art| art.filenames.iter().any(|f| path::Path::new(f).file_name() == exe.file_name()))
        .ok_or("Could not retrieve metadata")?;

//    let _build_bundles = if sub_args.is_present("DEBUGGER") {
        debug!("Debug app");
//        device.debug_app(&project, runnable, run_env, &*args, &*envs)?
//    } else {
        debug!("Run app");
        let runnable = dinghy_lib::Runnable {
            id: exe.file_name().unwrap().to_str().unwrap().to_string(), // both checked
            exe,
            src: artefact.target.src_path.parent().unwrap().parent().unwrap().into(),
        };
        let run_env = dinghy_lib::RunEnv {
            compile_mode: dinghy_lib::CompileMode::Test,
            rustc_triple: ctx.platform.rustc_triple().map(|s| s.to_string()),
            dynamic_libraries: vec!(), // FIXME
            args,
            envs,
        };
        device.run_app(&ctx.project, &runnable, &run_env)?;
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

fn show_all_devices(args:&[&OsStr]) -> Result<()> {
    let ctx = DinghyCtxt::new(args)?;
    println!("List of available devices for all platforms:");
    show_devices(&ctx.dinghy, None)
}

fn show_all_platforms(args:&[&OsStr]) -> Result<()> {
    let ctx = DinghyCtxt::new(args)?;
    for pf in ctx.dinghy.platforms() {
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
