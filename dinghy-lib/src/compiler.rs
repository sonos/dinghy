extern crate cargo;

use Build;
use BuildArgs;
use cargo::core::compiler as CargoCoreCompiler;
use cargo::core::compiler::Compilation;
pub use cargo::core::compiler::CompileMode;
use cargo::core::compiler::MessageFormat;
use cargo::core::Workspace;
use cargo::ops as CargoOps;
use cargo::ops::CleanOptions;
use cargo::ops::CompileFilter;
use cargo::ops::CompileOptions;
use cargo::ops::Packages as CompilePackages;
use cargo::ops::TestOptions;
use cargo::util::config::Config as CompileConfig;
use cargo::util::important_paths::find_root_manifest_for_wd;
use clap::ArgMatches;
use dinghy_build::build_env::target_env_from_triple;
use ErrorKind;
use itertools::Itertools;
use Result;
use ResultExt;
use Runnable;
use std::collections::HashSet;
use std::env;
use std::env::current_dir;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::iter::FromIterator;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use toml;
use utils::arg_as_string_vec;
use utils::copy_and_sync_file;
use utils::is_library;
use walkdir::WalkDir;

pub struct Compiler {
    build_command: Box<Fn(Option<&str>, &BuildArgs) -> Result<Build>>,
    clean_command: Box<Fn(Option<&str>) -> Result<()>>,
    run_command: Box<Fn(Option<&str>, &BuildArgs, &[&str]) -> Result<()>>,
}

impl Compiler {
    pub fn from_args(matches: &ArgMatches) -> Self {
        Compiler {
            build_command: create_build_command(matches),
            clean_command: create_clean_command(matches),
            run_command: create_run_command(matches),
        }
    }

    pub fn build(&self, rustc_triple: Option<&str>, build_args: &BuildArgs) -> Result<Build> {
        (self.build_command)(rustc_triple, build_args)
    }

    pub fn clean(&self, rustc_triple: Option<&str>) -> Result<()> {
        (self.clean_command)(rustc_triple)
    }

    pub fn run(&self, rustc_triple: Option<&str>, build_args: &BuildArgs, args: &[&str]) -> Result<()> {
        (self.run_command)(rustc_triple, build_args, args)
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
            || self.allowed_triples.contains(rustc_triple.unwrap_or("host")))
            && (self.ignored_triples.is_empty()
            || !self.ignored_triples.contains(rustc_triple.unwrap_or("host")))
    }
}

fn create_build_command(matches: &ArgMatches) -> Box<Fn(Option<&str>, &BuildArgs) -> Result<Build>> {
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
    let jobs = matches
        .value_of("JOBS")
        .map(|v| v.parse::<u32>().unwrap());
    let lib_only = matches.is_present("LIB");
    let no_default_features = matches.is_present("NO_DEFAULT_FEATURES");
    let packages = arg_as_string_vec(matches, "SPEC");
    let release = matches.is_present("RELEASE");
    let verbosity = matches.occurrences_of("VERBOSE") as u32;
    let tests = arg_as_string_vec(matches, "TEST");
    let bearded = matches.is_present("BEARDED");

    Box::new(move |rustc_triple: Option<&str>, build_args: &BuildArgs| {
        let release = build_args.compile_mode == CompileMode::Bench || release;
        let mut config = CompileConfig::default()?;
        config.configure(verbosity,
                         None,
                         &None,
                         false,
                         false,
                         &None,
                         &[])?;
        let workspace = Workspace::new(&find_root_manifest_for_wd(&current_dir()?)?,
                                       &config)?;

        let project_metadata_list = workskpace_metadata(&workspace)?;
        let filtered_projects = exclude_by_target_triple(rustc_triple,
                                                         project_metadata_list.as_slice(),
                                                         excludes.as_slice());

        // Note: exclude works only with all, hence this annoyingly convoluted condition...
        let (packages, excludes) = if (all || workspace.is_virtual()) && packages.is_empty() {
            (packages.clone(), filtered_projects)
        } else if workspace.is_virtual() && !packages.is_empty() {
            // Manual filtering in case we use -p as it doesn't work with exclude.
            // That avoids compiling the wrong project for the wrong platform.
            // This behaviour differs slightly from cargo itself
            let filtered_packages = packages.iter()
                .filter(|package| !filtered_projects.contains(package))
                .map(|it| it.to_string())
                .collect::<Vec<_>>();

            if filtered_packages.is_empty() {
                return Err(ErrorKind::PackagesCannotBeCompiledForPlatform(packages.clone()).into());
            } else {
                (filtered_packages, vec![]) // Exclude not allowed with -p, hence empty vec.
            }
        } else {
            (packages.clone(), excludes.clone())
        };

        let build_config = CargoCoreCompiler::BuildConfig {
            release,
            message_format: MessageFormat::Human,
            ..
            CargoCoreCompiler::BuildConfig::new(&config,
                                                jobs,
                                                &rustc_triple.map(str::to_string), build_args.compile_mode)?
        };

        let compile_options = CompileOptions {
            config: &config,
            build_config,
            features: features.clone(),
            all_features,
            no_default_features,
            spec: CompilePackages::from_flags(
                all,
                excludes,
                packages,
            )?,
            filter: CompileFilter::new(
                lib_only,
                bins.clone(), false,
                tests.clone(), false,
                examples.clone(), false,
                benches.clone(), false,
                false, // all_targets
            ),
            target_rustdoc_args: None,
            target_rustc_args: None,
            export_dir: None,
            local_rustdoc_args: None,
        };

        if bearded { setup_dinghy_wrapper(&workspace, rustc_triple)?; }
        let compilation = CargoOps::compile(&workspace, &compile_options)?;
        let build = to_build(compilation, &config, build_args, rustc_triple)?;
        copy_dependencies_to_target(&build)?;
        Ok(build)
    })
}

fn create_clean_command(matches: &ArgMatches) -> Box<Fn(Option<&str>) -> Result<()>> {
    let packages = arg_as_string_vec(matches, "SPEC");
    let release = matches.is_present("RELEASE");
    let verbosity = matches.occurrences_of("VERBOSE") as u32;

    Box::new(move |rustc_triple: Option<&str>| {
        let mut config = CompileConfig::default()?;
        config.configure(verbosity,
                         None,
                         &None,
                         false,
                         false,
                         &None,
                         &[])?;
        let workspace = Workspace::new(&find_root_manifest_for_wd(&current_dir()?)?,
                                       &config)?;

        let options = CleanOptions {
            config: &config,
            release,
            spec: packages.clone(),
            target: rustc_triple.map(str::to_string),
            doc: false,
        };

        CargoOps::clean(&workspace, &options)?;
        Ok(())
    })
}

fn create_run_command(matches: &ArgMatches) -> Box<Fn(Option<&str>, &BuildArgs, &[&str]) -> Result<()>> {
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
    let jobs = matches
        .value_of("JOBS")
        .map(|v| v.parse::<u32>().unwrap());
    let lib_only = matches.is_present("LIB");
    let no_default_features = matches.is_present("NO_DEFAULT_FEATURES");
    let packages = arg_as_string_vec(matches, "SPEC");
    let release = matches.is_present("RELEASE");
    let verbosity = matches.occurrences_of("VERBOSE") as u32;
    let tests = arg_as_string_vec(matches, "TEST");
    let bearded = matches.is_present("BEARDED");

    Box::new(move |rustc_triple: Option<&str>, build_args: &BuildArgs, args: &[&str]| {
        let release = build_args.compile_mode == CompileMode::Bench || release;
        let mut config = CompileConfig::default()?;
        config.configure(verbosity,
                         None,
                         &None,
                         false,
                         false,
                         &None,
                         &[])?;
        let workspace = Workspace::new(&find_root_manifest_for_wd(&current_dir()?)?,
                                       &config)?;

        let project_metadata_list = workskpace_metadata(&workspace)?;
        let excludes = if (all || workspace.is_virtual()) && packages.is_empty() {
            exclude_by_target_triple(rustc_triple,
                                     project_metadata_list.as_slice(),
                                     excludes.as_slice())
        } else { excludes.clone() };

        let build_config = CargoCoreCompiler::BuildConfig {
            release,
            message_format: MessageFormat::Human,
            ..
            CargoCoreCompiler::BuildConfig::new(&config,
                                                jobs,
                                                &rustc_triple.map(str::to_string), build_args.compile_mode)?
        };


        let compile_options = CompileOptions {
            config: &config,
            build_config,
            features: features.clone(),
            all_features,
            no_default_features,
            spec: CompilePackages::from_flags(
                all,
                excludes,
                packages.clone(),
            )?,
            filter: CompileFilter::new(
                lib_only,
                bins.clone(), false,
                tests.clone(), false,
                examples.clone(), false,
                benches.clone(), false,
                false, // all_targets
            ),

            target_rustdoc_args: None,
            target_rustc_args: None,
            export_dir: None,
            local_rustdoc_args: None,
        };

        let test_options = TestOptions {
            compile_opts: compile_options,
            no_run: false,
            no_fail_fast: false,
        };

        if bearded { setup_dinghy_wrapper(&workspace, rustc_triple)?; }
        match build_args.compile_mode {
            CompileMode::Bench => {
                if let Some(err) = CargoOps::run_benches(&workspace,
                                                         &test_options,
                                                         args.into_iter().map(|it| it.to_string()).collect_vec().as_slice())? {
                    bail!("An error occured: {:?}", err);
                };
            }
            CompileMode::Build => {
                if let Some(err) = CargoOps::run(&workspace,
                                                 &test_options.compile_opts,
                                                 args.into_iter().map(|it| it.to_string()).collect_vec().as_slice())? {
                    bail!("An error occured: {:?}", err);
                };
            }
            CompileMode::Test => {
                if let Some(err) = CargoOps::run_tests(&workspace,
                                                       &test_options,
                                                       args.into_iter().map(|it| it.to_string()).collect_vec().as_slice())? {
                    bail!("An error occured: {:?}", err);
                };
            }
            otherwise => {
                bail!("Invalid run option {:?}", otherwise);
            }
        }
        Ok(())
    })
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
        measure_sh.write_all(format!("echo \"$4 = $ELAPSED_TIME s\" >> {}\n", target_dir.join("dinghy-wrapper.log").display()).as_bytes())?;
    }
    fs::set_permissions(&measure_sh_path, PermissionsExt::from_mode(0o755))?;
    env::set_var("RUSTC_WRAPPER", measure_sh_path);
    Ok(())
}

fn copy_dependencies_to_target(build: &Build) -> Result<()> {
    for src_lib_path in &build.dynamic_libraries {
        let target_lib_path = build.target_path.join(src_lib_path.file_name()
            .ok_or(format!("Invalid file name {:?}", src_lib_path.file_name()))?);

        debug!("Copying dynamic lib {} to {}", src_lib_path.display(), target_lib_path.display());
        copy_and_sync_file(&src_lib_path, &target_lib_path)
            .chain_err(|| format!("Couldn't copy {} to {}", src_lib_path.display(), &target_lib_path.display()))?;
    }
    Ok(())
}

fn to_build(compilation: Compilation,
            config: &CompileConfig,
            build_args: &BuildArgs,
            rustc_triple: Option<&str>) -> Result<Build> {
    match build_args.compile_mode {
        CompileMode::Build => {
            Ok(Build {
                build_args: build_args.clone(),
                dynamic_libraries: find_dynamic_libraries(&compilation,
                                                          config,
                                                          build_args,
                                                          rustc_triple)?,
                runnables: compilation.binaries
                    .iter()
                    .map(|exe_path| {
                        Ok(Runnable {
                            exe: exe_path.clone(),
                            id: exe_path.file_name()
                                .ok_or(format!("Invalid executable file '{}'", &exe_path.display()))?
                                .to_str()
                                .ok_or(format!("Invalid executable file '{}'", &exe_path.display()))?
                                .to_string(),
                            source: PathBuf::from("."),
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
                target_path: compilation.root_output.clone(),
            })
        }

        _ => {
            Ok(Build {
                build_args: build_args.clone(),
                dynamic_libraries: find_dynamic_libraries(&compilation,
                                                          config,
                                                          build_args,
                                                          rustc_triple)?,
                runnables: compilation.tests
                    .iter()
                    .map(|&(ref pkg, _, _, ref exe_path)| {
                        Ok(Runnable {
                            exe: exe_path.clone(),
                            id: exe_path.file_name()
                                .ok_or(format!("Invalid executable file '{}'", &exe_path.display()))?
                                .to_str()
                                .ok_or(format!("Invalid executable file '{}'", &exe_path.display()))?
                                .to_string(),
                            source: pkg.root().to_path_buf(),
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
                target_path: compilation.root_output.clone(),
            })
        }
    }
}

fn exclude_by_target_triple(rustc_triple: Option<&str>, project_metadata_list: &[ProjectMetadata], excludes: &[String]) -> Vec<String> {
    let mut all_excludes: Vec<String> = excludes.to_vec();
    all_excludes.extend(project_metadata_list.iter()
        .filter(|metadata| !metadata.is_allowed_for(rustc_triple))
        .filter(|metadata| !excludes.contains(&metadata.project_id))
        .map(|metadata| {
            debug!("Project '{}' is disabled for current target", metadata.project_id);
            metadata.project_id.clone()
        }));
    all_excludes
}

// Try to find all linked libraries in (absolutely all for now) cargo output files
// and then look for the corresponding one in all library paths.
// Note: This looks highly imperfect and prone to failure (like if multiple version of
// the same dependency are available). Need improvement.
fn find_dynamic_libraries(compilation: &Compilation,
                          config: &CompileConfig,
                          build_args: &BuildArgs,
                          rustc_triple: Option<&str>) -> Result<Vec<PathBuf>> {
    let sysroot = match linker(compilation, config) {
        Ok(linker) => PathBuf::from(String::from_utf8(
            Command::new(&linker).arg("-print-sysroot")
                .output()
                .chain_err(|| format!("Error while checking libraries using linker {}", linker.display()))?
                .stdout)?.trim()),
        Err(err) => match rustc_triple {
            None => PathBuf::from(""), // Host platform case
            Some(_) => return Err(err),
        },
    };
    let linked_library_names = find_all_linked_library_names(compilation, build_args)?;

    let is_library_linked_to_project = move |path: &PathBuf| -> bool {
        path.file_name()
            .and_then(|file_name| file_name.to_str())
            .map(|file_name| linked_library_names.iter()
                .find(|lib_name| file_name == format!("lib{}.so", lib_name)
                    || file_name == format!("lib{}.dylib", lib_name)
                    || file_name == format!("lib{}.a", lib_name))
                .is_some())
            .unwrap_or(false)
    };

    let is_banned = move |path: &PathBuf| -> bool {
        path.file_name()
            .and_then(|file_name| file_name.to_str())
            .map(|file_name| file_name != "libstdc++.so" || !rustc_triple.map(|it| it.contains("android")).unwrap_or(false))
            .unwrap_or(false)
    };

    Ok(compilation.native_dirs.iter() // Should better use output files instead of deprecated native_dirs
        .map(strip_annoying_prefix)
        .chain(linker_lib_dirs(&compilation, config)?.into_iter())
        .chain(overlay_lib_dirs(rustc_triple)?.into_iter())
        .inspect(|path| debug!("Checking library path {}", path.display()))
        .filter(move |path| !is_system_path(sysroot.as_path(), path).unwrap_or(true))
        .inspect(|path| debug!("{} is not a system library path", path.display()))
        .flat_map(|path| WalkDir::new(path).into_iter())
        .filter_map(|walk_entry| walk_entry.map(|it| it.path().to_path_buf()).ok())
        .filter(|path| is_library(path) && is_library_linked_to_project(path))
        .filter(|path| is_banned(path))
        .inspect(|path| debug!("Found library {}", path.display()))
        .collect())
}

fn find_all_linked_library_names(compilation: &Compilation, build_args: &BuildArgs) -> Result<HashSet<String>> {
    fn is_output_file(file_path: &PathBuf) -> bool {
        file_path.is_file() && file_path.file_name()
            .and_then(|it| it.to_str())
            .map(|it| it == "output")
            .unwrap_or(false)
    }

    fn parse_lib_name(lib_name: String) -> String {
        lib_name.split("=").last().map(|it| it.to_string()).unwrap_or(lib_name)
    }

    let linked_library_names =
            WalkDir::new(&compilation.root_output)
                .into_iter()
                .filter_map(|walk_entry| walk_entry.map(|it| it.path().to_path_buf()).ok())
                .filter(is_output_file)
                .map(|output_file| CargoCoreCompiler::BuildOutput::parse_file(&output_file, "idontcare", &compilation.root_output, &compilation.root_output))
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

pub fn linker_lib_dirs(compilation: &Compilation, config: &CompileConfig) -> Result<Vec<PathBuf>> {
    let linker = linker(compilation, config);
    if linker.is_err() { return Ok(vec![]); }

    let linker = linker?;
    if !linker.exists() { return Ok(vec![]); }

    let output = String::from_utf8(Command::new(&linker)
        .arg("-print-search-dirs")
        .output()
        .chain_err(|| format!("Error while checking libraries using linker {}", linker.display()))?
        .stdout)?;

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

fn linker(compilation: &Compilation, compile_config: &CompileConfig) -> Result<PathBuf> {
    let linker = compile_config.get_path(&format!("target.{}.linker", compilation.target))?;
    if let Some(linker) = linker {
        let linker = linker.val;
        if linker.exists() {
            return Ok(linker);
        } else {
            bail!("Couldn't find target linker")
        }
    } else {
        bail!("Couldn't find target linker")
    }
}

fn project_metadata<P: AsRef<Path>>(path: P) -> Result<Option<ProjectMetadata>> {
    fn read_file_to_string(mut file: File) -> Result<String> {
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Ok(content)
    }

    let toml = File::open(&path.as_ref())
        .chain_err(|| format!("Couldn't open {}", path.as_ref().display()))
        .and_then(read_file_to_string)
        .and_then(|toml_content| toml_content.parse::<toml::Value>()
            .chain_err(|| format!("Couldn'parse {}", path.as_ref().display())))?;

    let project_id = toml.get("package")
        .and_then(|it| it.get("name"))
        .and_then(|it| it.as_str());

    let metadata = toml.get("package")
        .and_then(|it| it.get("metadata"))
        .and_then(|it| it.get("dinghy"));

    if let (Some(project_id), Some(metadata)) = (project_id, metadata) {
        Ok(Some(ProjectMetadata {
            project_id: project_id.to_string(),
            allowed_triples: HashSet::from_iter(metadata.get("allowed_rustc_triples")
                .and_then(|targets| targets.as_array())
                .unwrap_or(&vec![])
                .into_iter()
                .filter_map(|target| target.as_str().map(|it| it.to_string()))
                .collect_vec()),
            ignored_triples: HashSet::from_iter(metadata.get("ignored_rustc_triples")
                .and_then(|targets| targets.as_array())
                .unwrap_or(&vec![])
                .into_iter()
                .filter_map(|target| target.as_str().map(|it| it.to_string()))
                .collect_vec()),
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
                (Some("native"), Some(path)) |
                (Some("crate"), Some(path)) |
                (Some("dependency"), Some(path)) |
                (Some("framework"), Some(path)) |
                (Some("all"), Some(path)) => path.into(),
                _ => path.clone(),
            }
        }
        None => path.clone(),
    }
}

fn workskpace_metadata(workspace: &Workspace) -> Result<Vec<ProjectMetadata>> {
    workspace.members()
        .map(|member| project_metadata(member.manifest_path()))
        .filter_map(|metadata_res| match metadata_res {
            Err(error) => Some(Err(error)),
            Ok(metadata) => if let Some(metadata) = metadata { Some(Ok(metadata)) } else { None },
        })
        .collect::<Result<_>>()
}
