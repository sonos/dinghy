use std::{ env, fs, io, process };
use std::io::Write;
use std::collections::{ HashMap, HashSet };
use std::path::{ Path, PathBuf };

use rexpect::process::PtyProcess;
use std::os::unix::io::{FromRawFd, AsRawFd};
use nix::unistd::dup;
use dinghy_build::build_env::envify;

use walkdir::WalkDir;

use ::{ Build, BuildArgs, Result, ResultExt, Runnable };
use utils::{ GLOB_ARGS, is_library, create_shim, project_root };
use dinghy_build::build_env::target_env_from_triple;

#[derive(Debug, Serialize, Deserialize)]
pub struct CargoCompilerArtefact {
    pub filenames: Vec<PathBuf>,
    pub reason: String,
    pub profile: CargoCompilerArtefactProfile,
    pub target: CargoCompilerArtefactTarget,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CargoCompilerArtefactTarget {
    kind: Vec<String>,
    pub src_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CargoCompilerArtefactProfile {
    test: bool
}

pub fn call(build_args: &BuildArgs, rustc_triple:Option<&str>, mut env: HashMap<String,Option<String>>) -> Result<Build> {
    use std::io::Read;
    let artefacts_metadata = project_root()?.join("target").join("cargo-dinghy-current.json");
    let artefacts_metadata_tmp = artefacts_metadata.clone().with_extension("tmp");
    let mut cargo = process::Command::new("cargo");
    cargo.arg(&build_args.cargo_args[0]).arg("--message-format=json");
    if let Some(target) = rustc_triple {
        cargo.arg("--target").arg(target);
        if let Some(dev) = build_args.device.as_ref() {
            let path = create_shim(project_root()?, "runner", dev.id(), "runner", "runner", 
                                   &format!("{:?} runner --device {} -- {}", ::std::env::current_exe()?, dev.id(), GLOB_ARGS))?;
            env.insert(
                format!("CARGO_TARGET_{}_RUNNER", envify(target)),
                Some(path.to_string_lossy().to_string())
            );
        }
    } else {
        let path = create_shim(project_root()?, "runner", "host", "runner", "runner", 
                               &format!("{:?} runner -- {}", ::std::env::current_exe()?, GLOB_ARGS))?;
        env.insert(
            format!("CARGO_TARGET_{}_RUNNER", envify(::device::host::HOST_TRIPLE)),
            Some(path.to_string_lossy().to_string())
        );
    }
    let workspace = ::cargo_metadata::metadata(None)?;
    cargo.args(&build_args.cargo_args[1..]);
    for (k,v) in env {
        match v {
            Some(v) => cargo.env(k,v),
            None => cargo.env_remove(k),
        };
    }

    let process = PtyProcess::new(cargo)?;
    let mut f = process.get_file_handle();
    let mut buffer = vec!();
    let mut runnables = vec!();
    let mut artefacts = vec!();

    loop {
        let mut tmp = [0; 256];
        let n = f.read(&mut tmp)?;
        buffer.extend(&tmp[0..n]);
        let mut done = 0;
        while let Some(eol) = buffer[done..].iter().position(|&c| c == b'\n') {
            if buffer[done] == b'{' {
                if let Ok(artefact) = ::serde_json::from_reader::<_, CargoCompilerArtefact>(&buffer[done..done+eol]) {
                    if artefact.profile.test {
                        let exe_path = Path::new(&artefact.filenames[0]);
                        let mut source:&Path = artefact.target.src_path.as_path();
                        while !source.join("Cargo.toml").exists() {
                            source = source.parent().ok_or("no Cargo.toml found in package")?
                        }
                        runnables.push(Runnable {
                            id: exe_path.file_name().ok_or("test executable can not be a dir")?.to_string_lossy().to_string(),
                            exe: exe_path.to_owned(),
                            src: source.to_path_buf(),
                        });
                    }
                    artefacts.push(artefact);
                    {
                        let f = fs::File::create(&artefacts_metadata_tmp)?;
                        ::serde_json::to_writer(f, &artefacts)?;
                    }
                    fs::rename(&artefacts_metadata_tmp, &artefacts_metadata)?;
                }
            } else {
                io::stdout().write_all(&buffer[done..done+eol+1])?;
                io::stdout().flush()?;
            }
            done += eol+1;
        }
        buffer.drain(0..done);
        if n == 0 {
            break;
        }
    }

    let status = process.status();
    if let Some(::rexpect::process::wait::WaitStatus::Exited(_, code)) = status {
        if code != 0 {
            Err(::errors::ErrorKind::Child(code as _))?
        }
    }

    Ok(Build {
                build_args: build_args.clone(),
                dynamic_libraries: find_dynamic_libraries(artefacts,
                                                          "", // FIXME
                                                          build_args,
                                                          rustc_triple)?,
                target_path: "target/debug".into(), // FIXME
                rustc_triple: rustc_triple.map(|a| a.to_string()),
                runnables
    })
}

pub fn restore_artefacts_metadata() -> Result<Vec<CargoCompilerArtefact>> {
    use std::io::Read;
    let artefacts_metadata = project_root()?.join("target").join("cargo-dinghy-current.json");
    let f = fs::File::open(&artefacts_metadata)?;
    Ok(::serde_json::from_reader(f)?)
}

// Try to find all linked libraries in (absolutely all for now) cargo output files
// and then look for the corresponding one in all library paths.
// Note: This looks highly imperfect and prone to failure (like if multiple version of
// the same dependency are available). Need improvement.
fn find_dynamic_libraries(artefacts: Vec<CargoCompilerArtefact>,
                          root_output: &str,
                          build_args: &BuildArgs,
                          rustc_triple: Option<&str>) -> Result<Vec<PathBuf>> {
    let sysroot = match linker(rustc_triple) {
        Ok(linker) => PathBuf::from(String::from_utf8(
            process::Command::new(&linker).arg("-print-sysroot")
                .output()
                .chain_err(|| format!("Error while checking libraries using linker {}", linker.display()))?
                .stdout)?.trim()),
        Err(err) => match rustc_triple {
            None => PathBuf::from(""), // Host platform case
            Some(_) => return Err(err),
        },
    };
    let linked_library_names = find_all_linked_library_names(root_output, build_args)?;

    let is_library_linked_to_project = move |path: &Path| -> bool {
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

// FIXME
    Ok(
        artefacts.iter().flat_map(|art| art.filenames.iter())
    // compilation.native_dirs.iter() // Should better use output files instead of deprecated native_dirs
        .map(PathBuf::from)
        .map(strip_annoying_prefix)
//        .chain(linker_lib_dirs(&compilation, config)?.into_iter())
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

fn find_all_linked_library_names(root_output: &str, _build_args: &BuildArgs) -> Result<HashSet<String>> {
    /*
    fn is_output_file(file_path: &PathBuf) -> bool {
        file_path.is_file() && file_path.file_name()
            .and_then(|it| it.to_str())
            .map(|it| it == "output")
            .unwrap_or(false)
    }

    fn parse_lib_name(lib_name: String) -> String {
        lib_name.split("=").last().map(|it| it.to_string()).unwrap_or(lib_name)
    }
    */

    // FIXME linked libs
    Ok(HashSet::new())
    /*
    let linked_library_names =
        Itertools::flatten(
            WalkDir::new(root_output)
                .into_iter()
                .filter_map(|walk_entry| walk_entry.map(|it| it.path().to_path_buf()).ok())
                .filter(is_output_file)
                .map(|output_file| CargoOps::BuildOutput::parse_file(&output_file, "idontcare", &compilation.root_output, &compilation.root_output))
                .flat_map(|build_output| build_output.map(|it| it.library_links)))
            .map(|lib_name| lib_name.clone())
            .map(parse_lib_name)
            .chain(build_args.forced_overlays.clone())
            .collect();
    debug!("Found libraries {:?}", &linked_library_names);
    Ok(linked_library_names)
    */
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

// 💩💩💩 See cargo_rustc/mod.rs/filter_dynamic_search_path() 💩💩💩
fn strip_annoying_prefix(path: PathBuf) -> PathBuf {
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

pub fn overlay_lib_dirs(rustc_triple: Option<&str>) -> Result<Vec<PathBuf>> {
    let pkg_config_libdir = rustc_triple
        .map(|it| target_env_from_triple("PKG_CONFIG_LIBDIR", it, false).unwrap_or("".to_string()))
        .unwrap_or(env::var("PKG_CONFIG_LIBDIR").unwrap_or("".to_string()));

    Ok(pkg_config_libdir
        .split(":")
        .map(|it| PathBuf::from(it))
        .collect())
}

