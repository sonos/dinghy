use Build;
use BuildArgs;
use cargo_metadata::Metadata;
use clap::ArgMatches;
use dinghy_build::build_env::target_env_from_triple;
use Result;
use ResultExt;
use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use utils::copy_and_sync_file;
use utils::is_library;
use walkdir::WalkDir;
use Runnable;

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
    /*
    pub fn is_allowed_for(&self, rustc_triple: Option<&str>) -> bool {
        (self.allowed_triples.is_empty()
            || self.allowed_triples.contains(rustc_triple.unwrap_or("host")))
            && (self.ignored_triples.is_empty()
            || !self.ignored_triples.contains(rustc_triple.unwrap_or("host")))
    }
    */
}

fn create_build_command(matches: &ArgMatches) -> Box<Fn(Option<&str>, &BuildArgs) -> Result<Build>> {
    /*
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
    let lib_only = matches.is_present("LIB");
    let no_default_features = matches.is_present("NO_DEFAULT_FEATURES");
    let packages = arg_as_string_vec(matches, "SPEC");
    let release = matches.is_present("RELEASE");
    let verbosity = matches.occurrences_of("VERBOSE") as u32;
    let tests = arg_as_string_vec(matches, "TEST");
    */
    let bearded = matches.is_present("BEARDED");

    Box::new(move |rustc_triple: Option<&str>, build_args: &BuildArgs| {
        // Note: exclude works only with all, hence this annoyingly convoluted condition...
        // FIXME
        /*
        let release = build_args.compile_mode == CompileMode::Bench || release;
        let mut config = CompileConfig::default()?;
        config.configure(verbosity,
                         None,
                         &None,
                         false,
                         false,
                         &[])?;
        let workspace = Workspace::new(&find_root_manifest_for_wd(&current_dir()?)?,
                                       &config)?;

        let project_metadata_list = workskpace_metadata(&workspace)?;
        let filtered_projects = exclude_by_target_triple(rustc_triple,
                                                         project_metadata_list.as_slice(),
                                                         excludes.as_slice());

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
        */

        let workspace = ::cargo_metadata::metadata(None)?;
        if bearded { setup_dinghy_wrapper(&workspace, rustc_triple)?; }

        let mut root_output = Path::new(&workspace.workspace_root).join("target");
        let mut cargo = ::std::process::Command::new("cargo");
        cargo.arg(&build_args.cargo_args[0]).arg("--message-format=json");
        if let Some(target) = rustc_triple {
            cargo.arg("--target").arg(target);
            root_output.push(target);
        }
        if build_args.cargo_args[0] != OsString::from("build") {
            cargo.arg("--no-run");
        }
        cargo.args(&build_args.cargo_args[1..]);
        let cargo_output = cargo.output()?;
        if !cargo_output.status.success() {
            ::std::io::stdout().write_all(&cargo_output.stdout)?;
            ::std::io::stdout().write_all(&cargo_output.stderr)?;
            Err("cargo failed")?
        }
        let metadata = String::from_utf8(cargo_output.stdout)?;
        let build = to_build(metadata, &root_output, build_args, rustc_triple)?;
        copy_dependencies_to_target(&build)?;
        Ok(build)
    })
}

fn create_clean_command(_matches: &ArgMatches) -> Box<Fn(Option<&str>) -> Result<()>> {
    Box::new(move |_rustc_triple: Option<&str>| {
        let mut command = ::std::process::Command::new("cargo");
        command.arg("clean");
        if !command.status()?.success() {
            Err("cargo failed")?
        }
        Ok(())
    })
}

fn create_run_command(matches: &ArgMatches) -> Box<Fn(Option<&str>, &BuildArgs, &[&str]) -> Result<()>> {
    let bearded = matches.is_present("BEARDED");

    Box::new(move |rustc_triple: Option<&str>, build_args: &BuildArgs, _args: &[&str]| {
        let workspace = ::cargo_metadata::metadata(None)?;
        if bearded { setup_dinghy_wrapper(&workspace, rustc_triple)?; }
        let mut command = ::std::process::Command::new("cargo");
        command.args(&build_args.cargo_args);
        if !command.status()?.success() {
            Err("cargo failed")?
        }
        Ok(())
    })
}

fn setup_dinghy_wrapper(workspace: &Metadata, rustc_triple: Option<&str>) -> Result<()> {
    let mut target_dir = PathBuf::from(&workspace.target_directory);
    target_dir.push(rustc_triple.unwrap_or("host"));
    fs::create_dir_all(&target_dir)?;
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

    /*
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
                        Ok(Path {
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
                        Ok(Path {
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
    */
}

/*
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
*/


pub fn linker_lib_dirs(triple: Option<&str>) -> Result<Vec<PathBuf>> {
    let linker = linker(triple);
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
            let line = line.trim_left_matches("libraries: =");
            for path_str in line.split(":") {
                paths.push(PathBuf::from(path_str))
            }
        }
    }
    Ok(paths)
}

fn linker(_triple: Option<&str>) -> Result<PathBuf> {
    // FIXME
    Ok(::which::which("cc")?)
    /*
    let linker = compile_config.get_path(&format!("target.{}.linker", triple.unwrap_or("host")))?;
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
    */
}

/*
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
*/

/*
fn workskpace_metadata(workspace: &Workspace) -> Result<Vec<ProjectMetadata>> {
    workspace.members()
        .map(|member| project_metadata(member.manifest_path()))
        .filter_map(|metadata_res| match metadata_res {
            Err(error) => Some(Err(error)),
            Ok(metadata) => if let Some(metadata) = metadata { Some(Ok(metadata)) } else { None },
        })
        .collect::<Result<_>>()
}
*/
