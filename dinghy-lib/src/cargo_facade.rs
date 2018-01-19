pub use cargo::ops::CompileMode;
use cargo::util::important_paths::find_root_manifest_for_wd;
use cargo::core::Workspace;
use cargo::ops::CompileFilter;
use cargo::ops::CompileOptions;
use cargo::ops::MessageFormat;
use cargo::util::config::Config as CompileConfig;
use cargo::ops as CargoOps;
use cargo::ops::Packages as CompilePackages;
use clap::ArgMatches;
use itertools::Itertools;
use std::collections::HashSet;
use std::env::current_dir;
use std::fs::File;
use std::io::prelude::*;
use std::iter::FromIterator;
use std::path::Path;
use std::path::PathBuf;
use utils::arg_as_string_vec;
use toml;
use Platform;
use Result;
use ResultExt;
use Runnable;


pub struct CargoFacade {
    build_command: Box<Fn(&Platform, CompileMode) -> Result<Vec<Runnable>>>,
}

impl CargoFacade {
    pub fn from_args(matches: &ArgMatches) -> CargoFacade {
        CargoFacade {
            build_command: CargoFacade::create_build_command(matches),
        }
    }

    fn create_build_command(matches: &ArgMatches) -> Box<Fn(&Platform, CompileMode) -> Result<Vec<Runnable>>> {
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

        Box::new(move |platform: &Platform, compile_mode: CompileMode| {
            let release = compile_mode == CompileMode::Bench || release;
            let config = &CompileConfig::default()?;
            config.configure(
                verbosity,
                None,
                &None,
                false,
                false,
                &[],
            )?;
            let wd = Workspace::new(&find_root_manifest_for_wd(None, &current_dir()?)?,
                                    config)?;

            let project_metadata_list: Vec<ProjectMetadata> = wd.members()
                .map(|member| CargoFacade::read_project_metadata(member.manifest_path()))
                .filter_map(|metadata_res| match metadata_res {
                    Err(error) => Some(Err(error)),
                    Ok(metadata) => if let Some(metadata) = metadata { Some(Ok(metadata)) } else { None },
                })
                .collect::<Result<_>>()?;

            let mut all_excludes = excludes.clone();
            all_excludes.extend(project_metadata_list.iter()
                .filter(|metadata| !metadata.is_allowed_for(platform.rustc_triple()))
                .filter(|metadata| !excludes.contains(&metadata.project_id))
                .map(|metadata| {
                    debug!("Project '{}' is disabled for current target", metadata.project_id);
                    metadata.project_id.clone()
                }));

            let options = CompileOptions {
                config,
                jobs,
                target: platform.rustc_triple(),
                features: &*features,
                all_features,
                no_default_features,
                spec: CompilePackages::from_flags(
                    wd.is_virtual(),
                    all,
                    &all_excludes,
                    &packages,
                )?,
                filter: CompileFilter::new(
                    lib_only,
                    &bins, false,
                    &tests, false,
                    &examples, false,
                    &benches, false,
                    false, // all_targets
                ),
                release,
                mode: compile_mode,
                message_format: MessageFormat::Human,
                target_rustdoc_args: None,
                target_rustc_args: None,
            };

            let compilation = CargoOps::compile(&wd, &options)?;

            if compile_mode == CompileMode::Build {
                Ok(compilation
                    .binaries
                    .into_iter()
                    .take(1)
                    .map(|t| {
                        Runnable {
                            name: "main".into(),
                            exe: t,
                            source: PathBuf::from("."),
                        }
                    })
                    .collect::<Vec<_>>())
            } else {
                Ok(compilation
                    .tests
                    .into_iter()
                    .map(|(pkg, _, name, exe)| {
                        Runnable {
                            name: name,
                            source: pkg.root().to_path_buf(),
                            exe: exe,
                        }
                    })
                    .collect::<Vec<_>>())
            }
        })
    }

    fn read_project_metadata<P: AsRef<Path>>(path: P) -> Result<Option<ProjectMetadata>> {
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
                targets: HashSet::from_iter(metadata.get("allowed_rustc_triples")
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

    pub fn build(&self, platform: &Platform, compile_mode: CompileMode) -> Result<Vec<Runnable>> {
        (self.build_command)(platform, compile_mode)
    }

    pub fn project_dir(&self) -> Result<PathBuf> {
        let wd_path = ::cargo::util::important_paths::find_root_manifest_for_wd(None, &current_dir()?)?;
        Ok(wd_path.parent()
            .ok_or(format!("Couldn't read project directory {}.", wd_path.display()))?
            .to_path_buf())
    }

    pub fn target_dir(&self, rustc_triple: Option<&str>) -> Result<PathBuf> {
        let mut target_path = self.project_dir()?.join("target");
        if let Some(rustc_triple) = rustc_triple {
            target_path = target_path.join(rustc_triple);
        }
        Ok(target_path)
    }
}


#[derive(Debug, Default, Clone)]
struct ProjectMetadata {
    project_id: String,
    targets: HashSet<String>,
}

impl ProjectMetadata {
    pub fn is_allowed_for(&self, rustc_triple: Option<&str>) -> bool {
        self.targets.is_empty() || self.targets.contains(rustc_triple.unwrap_or("host"))
    }
}
