use clap::ArgMatches;
use std::path::PathBuf;
use std::path::Path;

pub fn arg_as_string_vec(matches: &ArgMatches, option: &str) -> Vec<String> {
    matches.values_of(option)
        .map(|vs| vs.map(|s| s.to_string()).collect())
        .unwrap_or(vec![])
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

pub fn path_between<P1: AsRef<Path>, P2: AsRef<Path>>(from: P1, to: P2) -> PathBuf {
    let mut path = PathBuf::from("/");
    for _ in from.as_ref() {
        path.push("..");
    }
    for dir in to.as_ref().iter().skip(1) {
        path.push(dir);
    }
    path
}
