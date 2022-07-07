use crate::cli::{DinghyCli, DinghyMode, DinghySubcommand, SubCommandWrapper};
use dinghy_lib::config::dinghy_config;
use dinghy_lib::errors::*;
use dinghy_lib::project::Project;
use dinghy_lib::Dinghy;
use dinghy_lib::Platform;
use dinghy_lib::{Build, SetupArgs};
use dinghy_lib::{Device, Runnable};
use std::env;
use std::env::current_dir;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time;

use log::{debug, error, info};
mod cli;

fn main() {
    let cli = DinghyCli::parse();

    if env::var("DINGHY_LOG").is_err() {
        let level_filter = match cli.args.verbose - cli.args.quiet {
            i8::MIN..=-1 => log::LevelFilter::Off,
            0 => log::LevelFilter::Error,
            1 => log::LevelFilter::Warn,
            2 => log::LevelFilter::Info,
            3 => log::LevelFilter::Debug,
            4..=i8::MAX => log::LevelFilter::Trace,
        };

        env_logger::Builder::new().filter_level(level_filter).init();
    } else {
        env_logger::init_from_env(
            env_logger::Env::new()
                .filter("DINGHY_LOG")
                .write_style("DINGHY_LOG_STYLE"),
        );
    }

    if let Err(e) = run_command(cli) {
        error!("{:?}", e);
        // positively ugly.
        if e.to_string().contains("are filtered out on platform") {
            std::process::exit(3)
        } else {
            std::process::exit(1)
        }
    }
}

fn run_command(cli: DinghyCli) -> Result<()> {
    let conf = Arc::new(dinghy_config(current_dir()?)?);

    let metadata = cargo_metadata::MetadataCommand::new().exec()?;

    let project = Project::new(&conf, metadata);
    let dinghy = Dinghy::probe(&conf)?;

    let (platform, device) = select_platform_and_device_from_cli(&cli, &dinghy)?;

    let setup_args = SetupArgs {
        verbosity: cli.args.verbose - cli.args.quiet,
        forced_overlays: cli.args.overlay.clone(),
        envs: cli.args.env.clone(),
        cleanup: cli.args.cleanup,
        strip: cli.args.strip, // TODO this should probably be configurable in the config as well
        device_id: device.as_ref().map(|d| d.id().to_string()),
    };

    match cli.mode {
        DinghyMode::CargoSubcommand { ref args } => {
            info!(
                "Targeting platform '{}' and device '{}'",
                platform.id(),
                device.as_ref().map(|it| it.id()).unwrap_or("<none>")
            );
            let cargo = env::var("CARGO")
                .map(PathBuf::from)
                .ok()
                .unwrap_or_else(|| PathBuf::from("cargo"));
            let mut cmd = Command::new(cargo);

            for arg in args {
                cmd.arg(arg);
            }

            platform.setup_env(&project, &setup_args)?;

            log::debug!("Launching {:?}", cmd);
            let status = cmd.status()?;
            log::debug!("done");

            std::process::exit(status.code().unwrap_or_else(|| {
                log::error!("Could not get cargo exit code");
                -1
            }));
        }
        DinghyMode::DinghySubcommand(DinghySubcommand::Runner { ref args }) => {
            debug!("starting dinghy runner, args {:?}", args);

            //let (platform, device) = select_platform_and_device_from_cli(&cli, &dinghy)?;

            if let Some(device) = device {
                let exe = args.first().cloned().unwrap();
                let exe_id = PathBuf::from(&exe)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
                let args_ref = args.iter().skip(1).map(|s| &s[..]).collect::<Vec<_>>();
                let envs_ref = cli.args.env.iter().map(|s| &s[..]).collect::<Vec<_>>();
                platform.setup_env(&project, &setup_args)?;

                dbg!(project
                    .metadata
                    .target_directory
                    .clone()
                    .join(platform.rustc_triple()));

                let mut build = Build {
                    setup_args,
                    // TODO these should be probably read from the executable file
                    dynamic_libraries: vec![],
                    runnable: Runnable {
                        id: exe_id,
                        exe: PathBuf::from(exe).canonicalize()?,
                        // cargo launches the runner inside the dir of the crate
                        source: PathBuf::from(".").canonicalize()?,
                    },
                    target_path: project.metadata.target_directory.clone().into(),
                };

                if cli.args.strip {
                    platform.strip(&mut build)?;
                }

                let bundle = device.run_app(
                    &project, &build, &args_ref,
                    &envs_ref, // TODO these are also in the SetupArgs
                )?;

                // TODO this is not done if the run fails
                if cli.args.cleanup {
                    device.clean_app(&bundle)?;
                }
            } else {
                bail!("No device")
            }
            Ok(())
        }
        DinghyMode::DinghySubcommand(DinghySubcommand::Devices {}) => {
            match cli
                .args
                .platform
                .as_ref()
                .map(|name| dinghy.platform_by_name(name))
            {
                None => anyhow::bail!("No platform provided"),
                Some(None) => anyhow::bail!("Unknown platform"),
                Some(Some(platform)) => show_all_devices_for_platform(&dinghy, platform),
            }
        }
        DinghyMode::DinghySubcommand(DinghySubcommand::AllDevices {}) => show_all_devices(&dinghy),
        DinghyMode::DinghySubcommand(DinghySubcommand::AllPlatforms {}) => {
            show_all_platforms(&dinghy)
        }
        DinghyMode::DinghySubcommand(DinghySubcommand::LldbProxy {}) => {
            let (_platform, device) = select_platform_and_device_from_cli(&cli, &dinghy)?;
            run_lldb(device)
        }
        DinghyMode::DinghySubcommand(DinghySubcommand::AllDinghySubcommands {}) => {
            use clap::CommandFactory;
            for sub in SubCommandWrapper::command().get_subcommands() {
                println!("{}\n\t{}", sub.get_name(), sub.get_about().unwrap_or(""));
            }
            Ok(())
        }
        DinghyMode::Naked => {
            anyhow::bail!("Naked mode") // what should we do?
        }
    }
}

fn run_lldb(device: Option<Arc<Box<dyn Device>>>) -> Result<()> {
    let device = device.ok_or_else(|| anyhow!("No device found"))?;
    let lldb = device.start_remote_lldb()?;
    info!("lldb running at: {}", lldb);
    loop {
        thread::sleep(time::Duration::from_millis(100));
    }
}

fn show_all_platforms(dinghy: &Dinghy) -> Result<()> {
    let mut platforms = dinghy.platforms();
    platforms.sort_by(|str1, str2| str1.id().cmp(&str2.id()));
    for pf in platforms.iter() {
        println!("* {} {}", pf.id(), pf.rustc_triple());
    }
    Ok(())
}

fn show_all_devices(dinghy: &Dinghy) -> Result<()> {
    println!("List of available devices for all platforms:");
    show_devices(&dinghy, None)
}

fn show_all_devices_for_platform(dinghy: &Dinghy, platform: Arc<Box<dyn Platform>>) -> Result<()> {
    println!(
        "List of available devices for platform '{}':",
        platform.id()
    );
    show_devices(&dinghy, Some(platform))
}

fn show_devices(dinghy: &Dinghy, platform: Option<Arc<Box<dyn Platform>>>) -> Result<()> {
    let devices = dinghy
        .devices()
        .into_iter()
        .filter(|device| {
            platform
                .as_ref()
                .map_or(true, |it| it.is_compatible_with(&***device))
        })
        .collect::<Vec<_>>();

    if devices.is_empty() {
        error!("No matching device found");
        println!("No matching device found");
    } else {
        for device in devices {
            let pf: Vec<_> = dinghy
                .platforms()
                .iter()
                .filter(|pf| pf.is_compatible_with(&**device))
                .cloned()
                .collect();
            println!("{}: {:?}", device, pf);
        }
    }
    Ok(())
}

fn select_platform_and_device_from_cli(
    cli: &DinghyCli,
    dinghy: &Dinghy,
) -> Result<(Arc<Box<dyn Platform>>, Option<Arc<Box<dyn Device>>>)> {
    if let Some(platform_name) = cli.args.platform.as_ref() {
        let platform = dinghy
            .platform_by_name(platform_name)
            .ok_or_else(|| anyhow!("No '{}' platform found", platform_name))?;

        let device = dinghy
            .devices()
            .into_iter()
            .filter(|device| {
                cli.args
                    .device
                    .as_ref()
                    .map(|filter| {
                        format!("{:?}", device)
                            .to_lowercase()
                            .contains(&filter.to_lowercase())
                    })
                    .unwrap_or(true)
            })
            .filter(|it| platform.is_compatible_with(&**it.as_ref()))
            .next();

        Ok((platform, device))
    } else if let Some(device_filter) = cli.args.device.as_ref() {
        let is_banned_auto_platform_id = |id: &str| -> bool {
            id.contains("auto-android")
                && (id.contains("min") || id.contains("latest") || id.contains("api"))
        };
        let devices = dinghy
            .devices()
            .into_iter()
            .filter(move |it| {
                format!("{:?}", it)
                    .to_lowercase()
                    .contains(&device_filter.to_lowercase())
            })
            .collect::<Vec<_>>();
        if devices.len() == 0 {
            bail!("No devices found for name hint `{}'", device_filter)
        }
        devices
            .into_iter()
            .filter_map(|d| {
                let pf = dinghy
                    .platforms()
                    .iter()
                    .filter(|pf| !is_banned_auto_platform_id(&pf.id()))
                    .find(|pf| pf.is_compatible_with(&**d))
                    .cloned();
                debug!(
                    "Looking for platform for {}: found {:?}",
                    d.id(),
                    pf.as_ref().map(|p| p.id())
                );
                pf.map(|it| (it, Some(d)))
            })
            .next()
            .ok_or_else(|| {
                anyhow!(
                    "No device and platform combination found for device hint `{}'",
                    device_filter
                )
            })
    } else {
        Ok((dinghy.host_platform(), None))
    }
}
