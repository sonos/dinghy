extern crate cargo;

use crate::utils::arg_as_string_vec;
use crate::utils::copy_and_sync_file;
use crate::utils::is_library;
use crate::Build;
use crate::BuildArgs;
use crate::Result;
use crate::Runnable;
use cargo::core::compiler as CargoCoreCompiler;
use cargo::core::compiler::Compilation;
use cargo::core::compiler::CompileKind;
pub use cargo::core::compiler::CompileMode;
use cargo::core::compiler::MessageFormat;
use cargo::core::Workspace;
use cargo::ops;
use cargo::ops::CleanOptions;
use cargo::ops::CompileFilter;
use cargo::ops::CompileOptions;
use cargo::ops::Packages as CompilePackages;
use cargo::ops::TestOptions;
use cargo::util::config::Config;
use cargo::util::important_paths::find_root_manifest_for_wd;
use cargo::util::interning::InternedString;
use clap::ArgMatches;
use dinghy_build::build_env::target_env_from_triple;
use itertools::Itertools;
use std::collections::HashSet;
use std::env;
use std::env::current_dir;
use std::ffi::OsString;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::iter::FromIterator;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use toml;
use walkdir::WalkDir;

use anyhow::Context;

use crate::Platform;

pub struct Compiler {
    build_command: Box<dyn Fn(&dyn Platform, &BuildArgs) -> Result<Build>>,
    clean_command: Box<dyn Fn(&dyn Platform) -> Result<()>>,
    run_command: Box<dyn Fn(&dyn Platform, &BuildArgs, &[&str]) -> Result<()>>,
}

impl Compiler {
    pub fn from_args(matches: &ArgMatches) -> Result<Self> {
        Ok(Compiler {
            build_command: create_build_command(matches)?,
            clean_command: create_clean_command(matches)?,
            run_command: create_run_command(matches)?,
        })
    }

    pub fn build(&self, platform: &dyn Platform, build_args: &BuildArgs) -> Result<Build> {
        (self.build_command)(platform, build_args)
    }

    pub fn clean(&self, platform: &dyn Platform) -> Result<()> {
        (self.clean_command)(platform)
    }

    pub fn run(
        &self,
        platform: &dyn Platform,
        build_args: &BuildArgs,
        args: &[impl AsRef<str>],
    ) -> Result<()> {
        let args = args.iter().map(AsRef::as_ref).collect::<Vec<_>>();
        (self.run_command)(platform, build_args, &*args)
    }
}

#[derive(Clone, Debug, Default)]
struct ProjectMetadata {
    project_id: String,
    allowed_triples: HashSet<String>,
    ignored_triples: HashSet<String>,
}

impl ProjectMetadata {
    pub fn is_allowed_for(&self, rustc_triple: Option<&str>) -> bool {
        (self.allowed_triples.is_empty()
            || self
                .allowed_triples
                .contains(rustc_triple.unwrap_or("host")))
            && (self.ignored_triples.is_empty()
                || !self
                    .ignored_triples
                    .contains(rustc_triple.unwrap_or("host")))
    }
}

fn config(matches: &ArgMatches) -> Result<Config> {
    let offline = matches.is_present("OFFLINE");
    let verbosity = matches.occurrences_of("VERBOSE") as u32;
    let mut config = Config::default()?;
    config.configure(
        verbosity,
        false,
        None,
        false,
        false,
        offline,
        &None,
        &[],
        &[],
    )?;
    Ok(config)
}

fn profile(release: bool, build_args: &BuildArgs) -> InternedString {
    if release || build_args.compile_mode == cargo::util::command_prelude::CompileMode::Bench {
        InternedString::new("release")
    } else {
        InternedString::new("debug")
    }
}

fn create_build_command(
    matches: &ArgMatches,
) -> Result<Box<dyn Fn(&dyn Platform, &BuildArgs) -> Result<Build>>> {
    let all = matches.is_present("ALL");
    let all_features = matches.is_present("ALL_FEATURES");
    let benches = arg_as_string_vec(matches, "BENCH");
    let bins = arg_as_string_vec(matches, "BIN");
    let features: Vec<String> = matches
        .value_of("FEATURES")
        .map(|f| f.split(" ").map(|s| s.into()).collect())
        .unwrap_or(vec![]);
    let examples = arg_as_string_vec(matches, "EXAMPLE");
    let excludes = arg_as_string_vec(matches, "EXCLUDE");
    let jobs = matches.value_of("JOBS").map(|v| v.parse::<u32>().unwrap());
    let lib_only = matches.is_present("LIB");
    let no_default_features = matches.is_present("NO_DEFAULT_FEATURES");
    let packages = arg_as_string_vec(matches, "SPEC");

    let release = matches.is_present("RELEASE");
    let tests = arg_as_string_vec(matches, "TEST");
    let bearded = matches.is_present("BEARDED");

    let config = config(matches)?;

    let f = Box::new(move |platform: &dyn Platform, build_args: &BuildArgs| {
        let requested_profile = profile(release, build_args);
        let root_manifest = find_root_manifest_for_wd(&current_dir()?)?;
        if current_dir()? == root_manifest.parent().unwrap() && features.len() > 0 {
            bail!("cargo does not support --features flag when building from root of workspace")
        }
        let workspace = Workspace::new(&root_manifest, &config)?;

        let project_metadata_list = workskpace_metadata(&workspace)?;
        let filtered_projects = exclude_by_target_triple(
            Some(&platform.rustc_triple().to_string()),
            project_metadata_list.as_slice(),
            excludes.as_slice(),
        );

        // Note: exclude works only with all, hence this annoyingly convoluted condition...
        let (packages, excludes) = if (all || workspace.is_virtual()) && packages.is_empty() {
            (packages.clone(), filtered_projects)
        } else if workspace.is_virtual() && !packages.is_empty() {
            // Manual filtering in case we use -p as it doesn't work with exclude.
            // That avoids compiling the wrong project for the wrong platform.
            // This behaviour differs slightly from cargo itself
            let filtered_packages = packages
                .iter()
                .filter(|package| !filtered_projects.contains(package))
                .map(|it| it.to_string())
                .collect::<Vec<_>>();

            if filtered_packages.is_empty() {
                bail!(
                    "packages {:?} are filtered out on platform {:?}",
                    packages,
                    platform
                )
            } else {
                (filtered_packages, vec![]) // Exclude not allowed with -p, hence empty vec.
            }
        } else {
            (packages.clone(), excludes.clone())
        };

        let mut build_config = CargoCoreCompiler::BuildConfig::new(
            &config,
            jobs,
            &[platform.rustc_triple().to_string()],
            build_args.compile_mode,
        )?;
        build_config.requested_kinds = vec![requested_kind];
        build_config.requested_profile = requested_profile;
        build_config.message_format = MessageFormat::Human;

        let compile_options = CompileOptions {
            build_config,
            features: features.clone(),
            all_features,
            no_default_features,
            spec: CompilePackages::from_flags(all, excludes, packages)?,
            filter: CompileFilter::from_raw_arguments(
                lib_only,
                bins.clone(),
                false,
                tests.clone(),
                false,
                examples.clone(),
                false,
                benches.clone(),
                false,
                false, // all_targets
            ),
            target_rustdoc_args: None,
            target_rustc_args: None,
            local_rustdoc_args: None,
            rustdoc_document_private_items: false,
        };
        if bearded {
            setup_dinghy_wrapper(&workspace, rustc_triple)?;
        }
        let compilation = ops::compile(&workspace, &compile_options)?;
        let build = to_build(compilation, &config, build_args, rustc_triple, sysroot)?;
        copy_dependencies_to_target(&build)?;
        Ok(build)
    });
    Ok(f)
}

fn create_clean_command(matches: &ArgMatches) -> Result<Box<dyn Fn(Option<&str>) -> Result<()>>> {
    let packages = arg_as_string_vec(matches, "SPEC");
    let release = matches.is_present("RELEASE");
    let config = config(matches)?;

    let f = Box::new(move |rustc_triple: Option<&str>| {
        let workspace = Workspace::new(&find_root_manifest_for_wd(&current_dir()?)?, &config)?;
        let requested_profile = InternedString::new(if release { "release" } else { "debug" });
        let (_, target) = kind_and_target(rustc_triple)?;

        let options = CleanOptions {
            config: &config,
            requested_profile,
            profile_specified: false,
            spec: packages.clone(),
            targets: vec![target],
            doc: false,
        };

        ops::clean(&workspace, &options)?;
        Ok(())
    });
    Ok(f)
}

fn create_run_command(
    matches: &ArgMatches,
) -> Result<Box<dyn Fn(Option<&str>, &BuildArgs, &[&str]) -> Result<()>>> {
    let all = matches.is_present("ALL");
    let all_features = matches.is_present("ALL_FEATURES");
    let benches = arg_as_string_vec(matches, "BENCH");
    let bins = arg_as_string_vec(matches, "BIN");
    let features: Vec<String> = matches
        .value_of("FEATURES")
        .unwrap_or("")
        .split(" ")
        .map(|s| s.into())
        .collect();
    let examples = arg_as_string_vec(matches, "EXAMPLE");
    let excludes = arg_as_string_vec(matches, "EXCLUDE");
    let jobs = matches.value_of("JOBS").map(|v| v.parse::<u32>().unwrap());
    let lib_only = matches.is_present("LIB");
    let no_default_features = matches.is_present("NO_DEFAULT_FEATURES");
    let packages = arg_as_string_vec(matches, "SPEC");

    let release = matches.is_present("RELEASE");
    let tests = arg_as_string_vec(matches, "TEST");
    let bearded = matches.is_present("BEARDED");
    let config = config(matches)?;

    let f = Box::new(
        move |rustc_triple: Option<&str>, build_args: &BuildArgs, args: &[&str]| {
            let workspace = Workspace::new(&find_root_manifest_for_wd(&current_dir()?)?, &config)?;

            let project_metadata_list = workskpace_metadata(&workspace)?;
            let excludes = if (all || workspace.is_virtual()) && packages.is_empty() {
                exclude_by_target_triple(
                    rustc_triple,
                    project_metadata_list.as_slice(),
                    excludes.as_slice(),
                )
            } else {
                excludes.clone()
            };
            let requested_profile = InternedString::new(if release { "release" } else { "debug" });
            let (kind, target) = kind_and_target(rustc_triple)?;

            let build_config = CargoCoreCompiler::BuildConfig {
                message_format: MessageFormat::Human,
                requested_kinds: vec![kind],
                requested_profile,
                ..CargoCoreCompiler::BuildConfig::new(
                    &config,
                    jobs,
                    &[target],
                    build_args.compile_mode,
                )?
            };

            let compile_options = CompileOptions {
                build_config,
                features: features.clone(),
                all_features,
                no_default_features,
                spec: CompilePackages::from_flags(all, excludes, packages.clone())?,
                filter: CompileFilter::from_raw_arguments(
                    lib_only,
                    bins.clone(),
                    false,
                    tests.clone(),
                    false,
                    examples.clone(),
                    false,
                    benches.clone(),
                    false,
                    false, // all_targets
                ),

                target_rustdoc_args: None,
                target_rustc_args: None,
                local_rustdoc_args: None,
                rustdoc_document_private_items: false,
            };

            let test_options = TestOptions {
                compile_opts: compile_options,
                no_run: false,
                no_fail_fast: false,
            };

            if bearded {
                setup_dinghy_wrapper(&workspace, rustc_triple)?;
            }
            match build_args.compile_mode {
                CompileMode::Bench => {
                    ops::run_benches(&workspace, &test_options, args)?;
                }
                CompileMode::Build => {
                    ops::run(
                        &workspace,
                        &test_options.compile_opts,
                        args.into_iter()
                            .map(|it| OsString::from(it))
                            .collect_vec()
                            .as_slice(),
                    )?;
                }
                CompileMode::Test => {
                    if let Some(err) = ops::run_tests(&workspace, &test_options, args)? {
                        Err(err)?;
                    }
                }
                otherwise => {
                    bail!("Invalid run option {:?}", otherwise);
                }
            }
            Ok(())
        },
    );
    Ok(f)
}

fn setup_dinghy_wrapper(workspace: &Workspace, rustc_triple: Option<&str>) -> Result<()> {
    let mut target_dir = workspace.target_dir();
    target_dir.push(rustc_triple.unwrap_or("host"));
    target_dir.create_dir()?;
    let target_dir = target_dir.into_path_unlocked();
    let measure_sh_path = target_dir.join("dinghy-wrapper.sh");
    {
        let mut measure_sh = File::create(&measure_sh_path)?;
        measure_sh.write_all(b"#!/bin/bash\n")?;
        measure_sh.write_all(b"START_TIME=$SECONDS\n")?;
        if let Ok(rustc_wrapper) = env::var("RUSTC_WRAPPER") {
            measure_sh.write_all(format!("(exec {} \"$@\")\n", rustc_wrapper).as_bytes())?;
        } else {
            measure_sh.write_all(b"(exec \"$@\")\n")?;
        }
        measure_sh.write_all(b"ELAPSED_TIME=$(($SECONDS - $START_TIME))\n")?;
        measure_sh.write_all(
            format!(
                "echo \"$4 = $ELAPSED_TIME s\" >> {}\n",
                target_dir.join("dinghy-wrapper.log").display()
            )
            .as_bytes(),
        )?;
    }
    #[cfg(unix)]
    fs::set_permissions(&measure_sh_path, PermissionsExt::from_mode(0o755))?;
    env::set_var("RUSTC_WRAPPER", measure_sh_path);
    Ok(())
}

fn copy_dependencies_to_target(build: &Build) -> Result<()> {
    for src_lib_path in &build.dynamic_libraries {
        let target_lib_path = build.target_path.join(
            src_lib_path
                .file_name()
                .ok_or_else(|| anyhow!("Invalid file name {:?}", src_lib_path.file_name()))?,
        );

        debug!(
            "Copying dynamic lib {} to {}",
            src_lib_path.display(),
            target_lib_path.display()
        );
        copy_and_sync_file(&src_lib_path, &target_lib_path).with_context(|| {
            format!(
                "Couldn't copy {} to {}",
                src_lib_path.display(),
                &target_lib_path.display()
            )
        })?;
    }
    Ok(())
}

fn to_build(
    compilation: Compilation,
    config: &Config,
    build_args: &BuildArgs,
    rustc_triple: Option<&str>,
    sysroot: Option<&str>,
) -> Result<Build> {
    let (kind, _) = kind_and_target(rustc_triple)?;
    match build_args.compile_mode {
        CompileMode::Build => Ok(Build {
            build_args: build_args.clone(),
            dynamic_libraries: find_dynamic_libraries(
                &compilation,
                config,
                build_args,
                rustc_triple,
                sysroot,
            )?,
            runnables: compilation
                .binaries
                .iter()
                .map(|exe_path| {
                    Ok(Runnable {
                        exe: exe_path.1.clone(),
                        id: exe_path
                            .1
                            .file_name()
                            .ok_or_else(|| {
                                anyhow!("Invalid executable file '{}'", &exe_path.1.display())
                            })?
                            .to_str()
                            .ok_or_else(|| {
                                anyhow!("Invalid executable file '{}'", &exe_path.1.display())
                            })?
                            .to_string(),
                        source: PathBuf::from("."),
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            target_path: compilation.root_output[&kind].clone(),
        }),

        _ => Ok(Build {
            build_args: build_args.clone(),
            dynamic_libraries: find_dynamic_libraries(
                &compilation,
                config,
                build_args,
                rustc_triple,
                sysroot,
            )?,
            runnables: compilation
                .tests
                .iter()
                .map(|&(ref u, ref exe_path)| {
                    Ok(Runnable {
                        exe: exe_path.clone(),
                        id: exe_path
                            .file_name()
                            .ok_or_else(|| {
                                anyhow!("Invalid executable file '{}'", &exe_path.display())
                            })?
                            .to_str()
                            .ok_or_else(|| {
                                anyhow!("Invalid executable file '{}'", &exe_path.display())
                            })?
                            .to_string(),
                        source: u.pkg.package_id().source_id().url().to_file_path().unwrap(),
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            target_path: compilation.root_output[&kind].clone(),
        }),
    }
}

fn exclude_by_target_triple(
    rustc_triple: Option<&str>,
    project_metadata_list: &[ProjectMetadata],
    excludes: &[String],
) -> Vec<String> {
    let mut all_excludes: Vec<String> = excludes.to_vec();
    all_excludes.extend(
        project_metadata_list
            .iter()
            .filter(|metadata| !metadata.is_allowed_for(rustc_triple))
            .filter(|metadata| !excludes.contains(&metadata.project_id))
            .map(|metadata| {
                debug!(
                    "Project '{}' is disabled for current target",
                    metadata.project_id
                );
                metadata.project_id.clone()
            }),
    );
    all_excludes
}

// Try to find all linked libraries in (absolutely all for now) cargo output files
// and then look for the corresponding one in all library paths.
// Note: This looks highly imperfect and prone to failure (like if multiple version of
// the same dependency are available). Need improvement.
fn find_dynamic_libraries(
    compilation: &Compilation,
    config: &Config,
    build_args: &BuildArgs,
    rustc_triple: Option<&str>,
    sysroot: Option<&Path>,
) -> Result<Vec<PathBuf>> {
    /*
    let sysroot = match linker(compilation, config) {
        Ok(linker) => PathBuf::from(
            String::from_utf8(
                Command::new(&linker)
                    .arg("-print-sysroot")
                    .output()
                    .with_context(|| {
                        format!(
                            "Error while checking libraries using linker {}",
                            linker.display()
                        )
                    })?
                    .stdout,
            )?
            .trim(),
        ),
        Err(err) => match rustc_triple {
            None => PathBuf::from(""), // Host platform case
            Some(triple) => return Err(err).with_context(|| format!("Looking for sysroot for {}", triple))
        },
    };
    */
    let linked_library_names =
        find_all_linked_library_names(compilation, build_args, rustc_triple)?;

    let is_library_linked_to_project = move |path: &PathBuf| -> bool {
        path.file_name()
            .and_then(|file_name| file_name.to_str())
            .map(|file_name| {
                linked_library_names
                    .iter()
                    .find(|lib_name| {
                        file_name == format!("lib{}.so", lib_name)
                            || file_name == format!("lib{}.dylib", lib_name)
                            || file_name == format!("lib{}.a", lib_name)
                    })
                    .is_some()
            })
            .unwrap_or(false)
    };

    let is_banned = move |path: &PathBuf| -> bool {
        path.file_name()
            .and_then(|file_name| file_name.to_str())
            .map(|file_name| {
                file_name != "libstdc++.so" && file_name != "libdl.so"
                    || !rustc_triple
                        .map(|it| it.contains("android"))
                        .unwrap_or(false)
            })
            .unwrap_or(false)
    };

    Ok(compilation
        .native_dirs
        .iter() // Should better use output files instead of deprecated native_dirs
        .map(strip_annoying_prefix)
        .chain(linker_lib_dirs(&compilation, config)?.into_iter())
        .chain(overlay_lib_dirs(rustc_triple)?.into_iter())
        .inspect(|path| trace!("Checking library path {}", path.display()))
        .filter(move |path| !is_system_path(sysroot.as_path(), path).unwrap_or(true))
        .inspect(|path| trace!("{} is not a system library path", path.display()))
        .flat_map(|path| WalkDir::new(path).into_iter())
        .filter_map(|walk_entry| walk_entry.map(|it| it.path().to_path_buf()).ok())
        .filter(|path| is_library(path) && is_library_linked_to_project(path))
        .filter(|path| is_banned(path))
        .fold(Vec::new(), |mut acc: Vec<PathBuf>, x| {
            if !acc
                .iter()
                .find(|x1| {
                    x.file_name().unwrap_or(&OsString::from(""))
                        == x1.file_name().unwrap_or(&OsString::from(""))
                })
                .is_some()
            {
                //If there is not yet a copy of the lib file in the vector
                acc.push(x);
                acc
            } else {
                acc
            }
        })
        .into_iter()
        .inspect(|path| debug!("Found library {}", path.display()))
        .collect())
}

fn find_all_linked_library_names(
    compilation: &Compilation,
    build_args: &BuildArgs,
    rustc_triple: Option<&str>,
) -> Result<HashSet<String>> {
    fn is_output_file(file_path: &PathBuf) -> bool {
        file_path.is_file()
            && file_path
                .file_name()
                .and_then(|it| it.to_str())
                .map(|it| it == "output")
                .unwrap_or(false)
    }

    fn parse_lib_name(lib_name: String) -> String {
        lib_name
            .split("=")
            .last()
            .map(|it| it.to_string())
            .unwrap_or(lib_name)
    }

    let (kind, _) = kind_and_target(rustc_triple)?;
    let root_output = &compilation.root_output[&kind];
    let linked_library_names = WalkDir::new(root_output)
        .into_iter()
        .filter_map(|walk_entry| walk_entry.map(|it| it.path().to_path_buf()).ok())
        .filter(is_output_file)
        .map(|output_file| {
            CargoCoreCompiler::BuildOutput::parse_file(
                &output_file,
                "idontcare",
                root_output,
                root_output,
            )
        })
        .flat_map(|build_output| build_output.map(|it| it.library_links))
        .flatten()
        .map(|lib_name| lib_name.clone())
        .map(parse_lib_name)
        .chain(build_args.forced_overlays.clone())
        .collect();
    debug!("Found libraries {:?}", &linked_library_names);
    Ok(linked_library_names)
}

fn is_system_path<P1: AsRef<Path>, P2: AsRef<Path>>(sysroot: P1, path: P2) -> Result<bool> {
    let ignored_path = vec![
        Path::new("/lib"),
        Path::new("/usr/lib"),
        Path::new("/usr/lib32"),
        Path::new("/usr/lib64"),
    ];
    let is_system_path = ignored_path.iter().any(|it| path.as_ref().starts_with(it));
    let is_sysroot_path = sysroot.as_ref().iter().count() > 0
        && path.as_ref().canonicalize()?.starts_with(sysroot.as_ref());
    Ok(is_system_path || is_sysroot_path)
}

pub fn linker_lib_dirs(compilation: &Compilation, config: &Config) -> Result<Vec<PathBuf>> {
    let linker = linker(compilation, config);
    if linker.is_err() {
        return Ok(vec![]);
    }

    let linker = linker?;
    if !linker.exists() {
        return Ok(vec![]);
    }

    let output = String::from_utf8(
        Command::new(&linker)
            .arg("-print-search-dirs")
            .output()
            .with_context(|| {
                format!(
                    "Error while checking libraries using linker {}",
                    linker.display()
                )
            })?
            .stdout,
    )?;

    let mut paths = vec![];
    for line in output.lines() {
        if line.starts_with("libraries: =") {
            let line = line.trim_start_matches("libraries: =");
            for path_str in line.split(":") {
                paths.push(PathBuf::from(path_str))
            }
        }
    }
    Ok(paths)
}

pub fn overlay_lib_dirs(rustc_triple: Option<&str>) -> Result<Vec<PathBuf>> {
    let pkg_config_libdir = rustc_triple
        .map(|it| target_env_from_triple("PKG_CONFIG_LIBDIR", it, false).unwrap_or("".to_string()))
        .unwrap_or(env::var("PKG_CONFIG_LIBDIR").unwrap_or("".to_string()));

    Ok(pkg_config_libdir
        .split(":")
        .map(|it| PathBuf::from(it))
        .collect())
}

fn linker(compilation: &Compilation, compile_config: &Config) -> Result<PathBuf> {
    let config = format!("target.{}.linker", compilation.host);
    let linker = compile_config.get_path(&config)?;
    if let Some(linker) = linker {
        let linker = linker.val;
        if linker.exists() {
            return Ok(linker);
        } else {
            bail!("Couldn't find target linker {}={:?}", config, linker)
        }
    } else {
        bail!("Couldn't find target linker {} not set.", config)
    }
}

fn project_metadata<P: AsRef<Path>>(path: P) -> Result<Option<ProjectMetadata>> {
    fn read_file_to_string(mut file: File) -> Result<String> {
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Ok(content)
    }

    let toml = File::open(&path.as_ref())
        .with_context(|| format!("Couldn't open {}", path.as_ref().display()))
        .and_then(read_file_to_string)
        .and_then(|toml_content| {
            toml_content
                .parse::<toml::Value>()
                .with_context(|| format!("Couldn'parse {}", path.as_ref().display()))
        })?;

    let project_id = toml
        .get("package")
        .and_then(|it| it.get("name"))
        .and_then(|it| it.as_str());

    let metadata = toml
        .get("package")
        .and_then(|it| it.get("metadata"))
        .and_then(|it| it.get("dinghy"));

    if let (Some(project_id), Some(metadata)) = (project_id, metadata) {
        Ok(Some(ProjectMetadata {
            project_id: project_id.to_string(),
            allowed_triples: HashSet::from_iter(
                metadata
                    .get("allowed_rustc_triples")
                    .and_then(|targets| targets.as_array())
                    .unwrap_or(&vec![])
                    .into_iter()
                    .filter_map(|target| target.as_str().map(|it| it.to_string()))
                    .collect_vec(),
            ),
            ignored_triples: HashSet::from_iter(
                metadata
                    .get("ignored_rustc_triples")
                    .and_then(|targets| targets.as_array())
                    .unwrap_or(&vec![])
                    .into_iter()
                    .filter_map(|target| target.as_str().map(|it| it.to_string()))
                    .collect_vec(),
            ),
        }))
    } else {
        Ok(None)
    }
}

// 💩💩💩 See cargo_rustc/mod.rs/filter_dynamic_search_path() 💩💩💩
fn strip_annoying_prefix(path: &PathBuf) -> PathBuf {
    match path.to_str() {
        Some(s) => {
            let mut parts = s.splitn(2, '=');
            match (parts.next(), parts.next()) {
                (Some("native"), Some(path))
                | (Some("crate"), Some(path))
                | (Some("dependency"), Some(path))
                | (Some("framework"), Some(path))
                | (Some("all"), Some(path)) => path.into(),
                _ => path.clone(),
            }
        }
        None => path.clone(),
    }
}

fn workskpace_metadata(workspace: &Workspace) -> Result<Vec<ProjectMetadata>> {
    workspace
        .members()
        .map(|member| project_metadata(member.manifest_path()))
        .filter_map(|metadata_res| match metadata_res {
            Err(error) => Some(Err(error)),
            Ok(metadata) => {
                if let Some(metadata) = metadata {
                    Some(Ok(metadata))
                } else {
                    None
                }
            }
        })
        .collect::<Result<_>>()
}
