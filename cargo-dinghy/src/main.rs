use std::convert::identity;
use std::env;
use std::env::current_dir;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use cargo_metadata::camino::Utf8PathBuf;
use cargo_metadata::Message;
use log::{debug, error, info};

use dinghy_lib::config::dinghy_config;
use dinghy_lib::errors::*;
use dinghy_lib::project::Project;
use dinghy_lib::utils::{set_current_verbosity, user_facing_log, LogCommandExt};
use dinghy_lib::Dinghy;
use dinghy_lib::Platform;
use dinghy_lib::{Build, SetupArgs};
use dinghy_lib::{Device, Runnable};

use crate::cli::{DinghyCli, DinghyMode, DinghySubcommand, SubCommandWrapper};

mod cli;

fn main() {
    let cli = DinghyCli::parse();

    //env::set_var("DINGHY_LOG", "trace");

    env_logger::init_from_env(
        env_logger::Env::new()
            .filter("DINGHY_LOG")
            .write_style("DINGHY_LOG_STYLE"),
    );

    set_current_verbosity(cli.args.verbose as i8);

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

    let workspace_root = metadata.workspace_root.into_std_path_buf();
    let target_directory = metadata.target_directory.into_std_path_buf();
    let project = Project::new(&conf, workspace_root, target_directory);
    let dinghy = Dinghy::probe(&conf)?;

    let (platform, device) = select_platform_and_device_from_cli(&cli, &dinghy)?;

    let setup_args = SetupArgs {
        verbosity: cli.args.verbose as i8 - cli.args.quiet as i8,
        forced_overlays: cli.args.overlay.clone(),
        envs: cli.args.env.clone(),
        cleanup: cli.args.cleanup,
        strip: cli.args.strip, // TODO this should probably be configurable in the config as well
        device_id: device.as_ref().map(|d| d.id().to_string()),
    };

    match cli.mode {
        DinghyMode::CargoSubcommand { ref args } => {
            let mut cmd = create_cargo_subcomand(&platform, &device, &project, &setup_args, args)?;

            log::debug!("Launching {:?}", cmd);
            let status = cmd.log_invocation(2).status()?;
            log::debug!("done");

            std::process::exit(status.code().unwrap_or_else(|| {
                log::error!("Could not get cargo exit code");
                -1
            }));
        }
        DinghyMode::DinghySubcommand(DinghySubcommand::Runner { ref args }) => {
            debug!("starting dinghy runner, args {:?}", args);

            let exe = args.first().cloned().unwrap();
            let exe_path = PathBuf::from(&exe);

            let inferred_target = exe_path
                .parent() // build type
                .and_then(|path| {
                    if path
                        .file_name()
                        .map(|name| name.to_string_lossy() == "deps")
                        .unwrap_or(false)
                    {
                        path.parent()
                    } else {
                        Some(path)
                    }
                })
                .and_then(Path::parent) // either "target" for the host or the actual target name if we're cross compiling
                .and_then(Path::file_name)
                .map(|it| it.to_string_lossy());

            debug!("inferred target {:?}", inferred_target);

            let (mut final_platform, mut final_device) = (platform, device);
            if let Some(inferred_target) = inferred_target {
                if final_device.is_none()
                    && final_platform.rustc_triple() != inferred_target
                    && cli.args.platform.is_none()
                {
                    let platform = dinghy
                        .platforms()
                        .into_iter()
                        .find(|p| p.rustc_triple() == inferred_target);
                    if let Some(platform) = platform {
                        let device = find_first_device_for_platform(&cli, &dinghy, &platform);
                        if let Some(device) = device {
                            info!("Runner was called without explicit platform, we found {} and device {}", platform.id(), device.id());
                            final_device = Some(device)
                        }
                        final_platform = platform;
                    }
                }
            };

            if let Some(device) = final_device {
                user_facing_log(
                    "Targeting",
                    &format!(
                        "platform {} and device {}",
                        final_platform.id(),
                        device.id()
                    ),
                    0,
                );
                let exe_id = exe_path.file_name().unwrap().to_str().unwrap().to_string();

                let (args, files_in_run_args): (Vec<String>, Vec<Option<PathBuf>>) = args
                    .into_iter()
                    .skip(1)
                    .map(|arg| {
                        if arg.contains(std::path::MAIN_SEPARATOR) {
                            let path_buf = PathBuf::from(&arg);
                            if path_buf.exists() {
                                (
                                    PathBuf::from(".")
                                        .join(path_buf.file_name().unwrap())
                                        .to_str()
                                        .unwrap()
                                        .to_string(),
                                    Some(path_buf),
                                )
                            } else {
                                (arg.clone(), None)
                            }
                        } else {
                            (arg.clone(), None)
                        }
                    })
                    .unzip();

                let files_in_run_args =
                    files_in_run_args.into_iter().filter_map(identity).collect();

                let args_ref = args.iter().map(|s| &s[..]).collect::<Vec<_>>();
                let envs_ref = cli.args.env.iter().map(|s| &s[..]).collect::<Vec<_>>();
                final_platform.setup_env(&project, &setup_args)?;

                let mut build = Build {
                    setup_args,
                    // TODO these should be probably read from the executable file
                    dynamic_libraries: vec![],
                    runnable: Runnable {
                        id: exe_id,
                        package_name: std::env::var("CARGO_PKG_NAME")?,
                        exe: PathBuf::from(exe).canonicalize()?,
                        // cargo launches the runner inside the dir of the crate
                        source: PathBuf::from(".").canonicalize()?,
                    },
                    target_path: project.target_directory.clone(),
                    files_in_run_args,
                };

                if cli.args.strip {
                    final_platform.strip(&mut build)?;
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
                bail!("No device for platform {}", final_platform.id())
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
        DinghyMode::DinghySubcommand(DinghySubcommand::AllDinghySubcommands {}) => {
            use clap::CommandFactory;
            for sub in SubCommandWrapper::command().get_subcommands() {
                println!(
                    "{}\n\t{}",
                    sub.get_name(),
                    sub.get_about().unwrap_or_default()
                );
            }
            Ok(())
        }
        DinghyMode::DinghySubcommand(DinghySubcommand::RunWith {
            wrapper_crate,
            mut lib_build_args,
        }) => {
            let mut build_command = vec!["build".to_string(), "--message-format=json".to_string()];
            build_command.append(&mut lib_build_args);

            let mut build_cargo_cmd =
                create_cargo_subcomand(&platform, &device, &project, &setup_args, &build_command)?;

            log::debug!("Launching {:?}", build_cargo_cmd);
            let mut child = build_cargo_cmd
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .log_invocation(2)
                .spawn()?;
            log::debug!("done");

            let mut extra_libs = vec![];

            let mut lib_file =
                cargo_metadata::Message::parse_stream(BufReader::new(child.stdout.take().unwrap()))
                    .filter_map(|message| match message {
                        Ok(Message::CompilerArtifact(artifact)) => {
                            if artifact.target.kind.contains(&"dylib".to_string()) {
                                extra_libs.append(&mut artifact.filenames.clone())
                            }
                            Some(artifact)
                        }
                        Ok(Message::CompilerMessage(message)) => {
                            // TODO would be really nice to get color there but current version of
                            // TODO cargo-metadata doesn't seem to support it
                            eprintln!("{}", message.message);
                            None
                        }
                        Ok(Message::BuildFinished(build)) => {
                            if !build.success {
                                log::debug!("cargo reported a build failure");
                            }
                            None
                        }
                        Ok(Message::TextLine(text)) => {
                            eprintln!("{}", text);
                            None
                        }
                        _ => None,
                    })
                    .last()
                    .ok_or_else(|| anyhow!("cargo did not produce an artifact"))?
                    .filenames
                    .into_iter()
                    .next()
                    .ok_or_else(|| anyhow!("no file in cargo artifact"))?;

            if !extra_libs.is_empty() {
                // if we have some extra libs, this means we're dealing with a dynamically linked
                // lib and we need to include the rust stdlib as well, let's try and find it in the
                // rustup home
                if let (Ok(rustup_home), Ok(rustup_toolchain)) = (
                    std::env::var("RUSTUP_HOME"),
                    std::env::var("RUSTUP_TOOLCHAIN"),
                ) {
                    let stdlib_dir = PathBuf::from(rustup_home)
                        .join("toolchains")
                        .join(rustup_toolchain)
                        .join("lib")
                        .join("rustlib")
                        .join(platform.rustc_triple())
                        .join("lib");

                    if let Ok(Some(stdlib)) = stdlib_dir.read_dir().map(|dir| {
                        dir.filter_map(|entry| {
                            entry
                                .and_then(|entry| {
                                    entry.file_type().map(|file_type| {
                                        if file_type.is_file()
                                            && entry
                                                .path()
                                                .file_stem()
                                                .map(|stem| {
                                                    stem.to_string_lossy().starts_with("libstd-")
                                                })
                                                .unwrap_or(false)
                                            && entry
                                                .path()
                                                .extension()
                                                .map(|ext| {
                                                    ["so", "dll", "dylib"]
                                                        .contains(&ext.to_string_lossy().as_ref())
                                                })
                                                .unwrap_or(false)
                                        {
                                            Some(entry.path())
                                        } else {
                                            None
                                        }
                                    })
                                })
                                .unwrap_or(None)
                        })
                        .next()
                    }) {
                        if let Ok(stdlib_path_buf) = Utf8PathBuf::from_path_buf(stdlib) {
                            extra_libs.push(stdlib_path_buf)
                        }
                    }
                }
            }

            let code = child.wait()?.code();

            match code {
                Some(0) => { /*expected*/ }
                Some(c) => std::process::exit(c),
                None => std::process::exit(-1),
            }

            if cli.args.strip {
                let stripped_dir = lib_file
                    .parent()
                    .ok_or_else(|| anyhow!("failed to get lib dir"))?
                    .join("stripped");

                std::fs::create_dir_all(&stripped_dir)?;

                let stripped_lib_file = stripped_dir.join(
                    lib_file
                        .file_name()
                        .ok_or_else(|| anyhow!("failed to get lib name"))?,
                );

                std::fs::copy(lib_file, &stripped_lib_file)?;

                let mut lib_build = Build {
                    setup_args: setup_args.clone(),
                    dynamic_libraries: vec![],
                    runnable: Runnable {
                        id: "".to_string(),
                        package_name: "".to_string(),
                        exe: stripped_lib_file.to_path_buf().into(),
                        source: Default::default(),
                    },
                    target_path: Default::default(),
                    files_in_run_args: vec![],
                };
                platform.strip(&mut lib_build)?;

                std::fs::copy(lib_build.runnable.exe, &stripped_lib_file)?;

                lib_file = stripped_lib_file;
            }

            let mut args = vec![
                "run".to_string(),
                "-p".to_string(),
                wrapper_crate,
                "--release".to_string(),
                "--".to_string(),
                lib_file.to_string(),
            ];
            for extra_lib in extra_libs {
                args.push(extra_lib.to_string())
            }
            let mut run_cargo_cmd =
                create_cargo_subcomand(&platform, &device, &project, &setup_args, &args)?;

            log::debug!("Launching {:?}", run_cargo_cmd);
            let status = run_cargo_cmd.log_invocation(2).status()?;
            log::debug!("done");

            std::process::exit(status.code().unwrap_or_else(|| {
                log::error!("Could not get cargo exit code");
                -1
            }));
        }
        DinghyMode::Naked => {
            anyhow::bail!("Naked mode") // what should we do?
        }
    }
}

fn create_cargo_subcomand(
    platform: &Arc<Box<dyn Platform>>,
    device: &Option<Arc<Box<dyn Device>>>,
    project: &Project,
    setup_args: &SetupArgs,
    args: &Vec<String>,
) -> Result<Command> {
    info!(
        "Targeting platform '{}' and device '{}'",
        platform.id(),
        device.as_ref().map(|it| it.id()).unwrap_or("<none>")
    );

    user_facing_log(
        "Targeting",
        &format!(
            "platform {} and device {}",
            platform.id(),
            device.as_ref().map(|it| it.id()).unwrap_or("<none>")
        ),
        0,
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
    Ok(cmd)
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

        let device = find_first_device_for_platform(cli, dinghy, &platform);

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

fn find_first_device_for_platform(
    cli: &DinghyCli,
    dinghy: &Dinghy,
    platform: &Arc<Box<dyn Platform>>,
) -> Option<Arc<Box<dyn Device>>> {
    dinghy
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
        .next()
}
