use clap::ArgMatches;
use errors::Result;
use filetime::set_file_times;
use filetime::FileTime;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub fn arg_as_string_vec(matches: &ArgMatches, option: &str) -> Vec<String> {
    matches.values_of(option)
        .map(|vs| vs.map(|s| s.to_string()).collect())
        .unwrap_or(vec![])
}

pub fn copy_and_sync_file<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<()> {
    let from = &from.as_ref();
    let to = &to.as_ref();
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
