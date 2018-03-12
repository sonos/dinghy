//! Some helpers around Path and PathBuf manipulations.

use std::path::Path;
use std::path::PathBuf;
use super::Result;

/// Wraps the annoying PathBuf to string conversion in one single call.
pub fn path_to_str(path: &PathBuf) -> Result<&str> {
    Ok(path.to_str().ok_or(format!("Not a valid UTF-8 path ({})", path.display()))?)
}

/// Finds the path to `to` relative from `from`.
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
