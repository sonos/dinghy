pub use cargo::ops::CompileMode;
use cargo::util::important_paths::find_root_manifest_for_wd;
use cargo::core::Workspace;
use cargo::ops::Compilation;
use cargo::ops::CompileFilter;
use cargo::ops::CompileOptions;
use cargo::ops::MessageFormat;
use cargo::ops::Packages as CompilePackages;
use cargo::ops as CargoOps;
use cargo::util::config::Config as CompileConfig;
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
use Result;
use ResultExt;
use Runnable;

pub struct Compiler {
    build_command: Box<Fn(Option<&str>, CompileMode) -> Result<CompilationResult>>,
}

impl Compiler {
    pub fn from_args(matches: &ArgMatches) -> Compiler {
        Compiler {
            build_command: Compiler::create_build_command(matches),
        }
    }

    fn create_build_command(matches: &ArgMatches) -> Box<Fn(Option<&str>, CompileMode) -> Result<CompilationResult>> {
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

        Box::new(move |rustc_triple: Option<&str>, compile_mode: CompileMode| {
            let release = compile_mode == CompileMode::Bench || release;
            let mut config = CompileConfig::default()?;
            config.configure(
                verbosity,
                None,
                &None,
                false,
                false,
                &[],
            )?;
            let workspace = Workspace::new(&find_root_manifest_for_wd(None, &current_dir()?)?,
                                           &config)?;

            let project_metadata_list = Compiler::workskpace_metadata(&workspace)?;
            let excludes = if all {
                Compiler::exclude_by_target_triple(rustc_triple,
                                                   project_metadata_list.as_slice(),
                                                   excludes.as_slice())
            } else { excludes.clone() };

            let options = CompileOptions {
                config: &config,
                jobs,
                target: rustc_triple,
                features: &*features,
                all_features,
                no_default_features,
                spec: CompilePackages::from_flags(
                    workspace.is_virtual(),
                    all,
                    &excludes,
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

            let compilation = CargoOps::compile(&workspace, &options)?;
            error!("TTTTTTTTT {:?}", compilation);
            Ok(Compiler::to_compilation_result(compilation, compile_mode)?)
        })
    }

    fn to_compilation_result(compilation: Compilation,
                             compile_mode: CompileMode) -> Result<CompilationResult> {
        let lib_directories = compilation.native_dirs
            .iter()
            .map(Compiler::strip_annoying_prefix)
            .collect_vec();

        if compile_mode == CompileMode::Build {
            Ok(CompilationResult {
                native_dirs: lib_directories,
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
        } else {
            Ok(CompilationResult {
                native_dirs: lib_directories,
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

    pub fn build(&self, rustc_triple: Option<&str>, compile_mode: CompileMode) -> Result<CompilationResult> {
        (self.build_command)(rustc_triple, compile_mode)
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

    fn workskpace_metadata(workspace: &Workspace) -> Result<Vec<ProjectMetadata>> {
        workspace.members()
            .map(|member| Compiler::project_metadata(member.manifest_path()))
            .filter_map(|metadata_res| match metadata_res {
                Err(error) => Some(Err(error)),
                Ok(metadata) => if let Some(metadata) = metadata { Some(Ok(metadata)) } else { None },
            })
            .collect::<Result<_>>()
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
}

#[derive(Clone, Debug, Default)]
pub struct CompilationResult {
    pub native_dirs: Vec<PathBuf>,
    pub runnables: Vec<Runnable>,
    pub target_path: PathBuf,
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
