use std::path::PathBuf;
use super::Result;

pub fn path_to_str(path: &PathBuf) -> Result<&str> {
    Ok(path.to_str().ok_or(format!("Not a valid UTF-8 path ({})", path.display()))?)
}
