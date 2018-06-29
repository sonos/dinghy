use clap::ArgMatches;
use errors::Result;
use filetime::FileTime;
use filetime::set_file_times;
use std::fs;
use std::path::{ Path, PathBuf };
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(not(target_os = "windows"))]
pub static GLOB_ARGS: &str = r#""$@""#;
#[cfg(target_os = "windows")]
pub static GLOB_ARGS: &str = r#"%*"#;


pub fn arg_as_string_vec(matches: &ArgMatches, option: &str) -> Vec<String> {
    matches.values_of(option)
        .map(|vs| vs.map(|s| s.to_string()).collect())
        .unwrap_or(vec![])
}

pub fn copy_and_sync_file<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<()> {
    let from = &from.as_ref();
    let to = &to.as_ref();

    // Make target file writeable if it is read-only.
    if to.exists() {
        let mut permissions = fs::metadata(&to)?.permissions();
        if permissions.readonly() {
            permissions.set_readonly(false);
            fs::set_permissions(&to, permissions)?;
        }
    }

    trace!("copy {:?} to {:?}", from, to);
    fs::copy(&from, &to)?;

    // Keep filetime to avoid useless sync on some devices (e.g. Android).
    let from_metadata = from.metadata()?;
    let atime = FileTime::from_last_access_time(&from_metadata);
    let mtime = FileTime::from_last_modification_time(&from_metadata);
    set_file_times(&to, atime, mtime)?;

    Ok(())
}

pub fn path_to_str<'a>(path: &'a Path) -> Result<&'a str> {
    Ok(path.to_str().ok_or(format!("Path is invalid '{}'", path.display()))?)
}

pub fn contains_file_with_ext(dir_path: &Path, ext: &str) -> bool {
    if !dir_path.is_dir() { return false; };
    if let Ok(path) = dir_path.read_dir() {
        for file in path {
            if let Ok(file) = file {
                if file.file_name().to_string_lossy().ends_with(ext) {
                    return true;
                }
            }
        }
    }
    false
}

pub fn destructure_path<P: AsRef<Path>>(path: P) -> Option<(PathBuf, String)> {
    let path = path.as_ref();
    path.file_name()
        .and_then(|it| it.to_str())
        .map(|name| (path.to_path_buf(), name.to_string()))
}

pub fn file_has_ext(file_path: &Path, ext: &str) -> bool {
    file_path.is_file() && file_path.file_name()
        .and_then(|it| it.to_str())
        .map(|it| it.ends_with(ext))
        .unwrap_or(false)
}

pub fn is_library(file_path: &Path) -> bool {
    file_path.is_file() && file_path.file_name()
        .and_then(|it| it.to_str())
        .map(|it| it.ends_with(".so")
            || it.contains(".so.")
            || it.ends_with(".dylib")
            || it.ends_with(".a"))
        .unwrap_or(false)
}

pub fn lib_name_from(file_path: &Path) -> Result<String> {
    let file_name = file_path.file_name()
        .and_then(|it| it.to_str())
        .ok_or(format!("'{}' doesn't point to a valid lib name", file_path.display()))?;

    let (start_index, end_index) = file_name.find(".so")
        .map(|end_index| (if file_name.starts_with("lib") { 3 } else { 0 }, end_index))
        .unwrap_or((0, file_name.len()));

    if start_index == end_index {
        bail!("'{}' doesn't point to a valid lib name", file_path.display());
    } else {
        Ok(file_name[start_index..end_index].to_string())
    }
}

pub fn file_name_as_str(file_path: &Path) -> Result<&str> {
    Ok(file_path.file_name()
        .and_then(|it| it.to_str())
        .ok_or(format!("'{}' is not a valid file name", file_path.display()))?)
}

pub fn create_shim<P: AsRef<Path>>(
    root: P,
    kind: &str,
    scope: &str,
    id: &str,
    name: &str,
    shell: &str,
) -> Result<PathBuf> {
    use std::io::Write;
    let target_shim_path = root.as_ref().join("target").join(kind).join(scope).join(id);
    fs::create_dir_all(&target_shim_path)?;
    let mut shim = target_shim_path.join(name);
    if cfg!(target_os = "windows") {
        shim.set_extension("bat");
    };
    let mut linker_shim = fs::File::create(&shim)?;
    if !cfg!(target_os = "windows") {
        writeln!(linker_shim, "#!/bin/sh")?;
    }
    linker_shim.write_all(shell.as_bytes())?;
    writeln!(linker_shim, "\n")?;
    if !cfg!(target_os = "windows") {
        fs::set_permissions(&shim, PermissionsExt::from_mode(0o777))?;
    }
    Ok(shim)
}

pub fn cargo_metadata<'a>() -> Result<&'a ::cargo_metadata::Metadata> {
    use std::sync::{Once, ONCE_INIT};
    unsafe {
        static START: Once = ONCE_INIT;
        static mut IT: Option<::std::result::Result<::cargo_metadata::Metadata, String>> = None;

        START.call_once(|| {
            IT = Some(::cargo_metadata::metadata(None)
                      .map_err(|e| format!("Can not read cargo metadata: {}", e)));
        });

        Ok(IT.as_ref().unwrap().as_ref().map_err(|s| s.clone())?)
    }
}

pub fn project_root() -> Result<PathBuf> {
    Ok(PathBuf::from(cargo_metadata()?.workspace_root.clone()))
}
