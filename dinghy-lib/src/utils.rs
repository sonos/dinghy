use crate::errors::Result;
use anyhow::Context;
use anyhow::{anyhow, bail};
use filetime::set_file_times;
use filetime::FileTime;
use lazy_static::lazy_static;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicI8, Ordering};

pub fn copy_and_sync_file<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<()> {
    let from = &from.as_ref();
    let to = &to.as_ref();

    if !from.exists() {
        bail!("Source {from:?} is missing")
    }

    if !to.parent().unwrap().exists() {
        bail!("Target directory is missing")
    }

    // Make target file writeable if it is read-only.
    if to.exists() {
        let mut permissions = fs::metadata(&to)
            .with_context(|| format!("Checking metadata for {to:?}"))?
            .permissions();
        if permissions.readonly() {
            permissions.set_readonly(false);
            fs::set_permissions(&to, permissions)
                .with_context(|| format!("Setting permissions {to:?}"))?;
        }
    }

    log::trace!("copy {:?} to {:?}", from, to);
    fs::copy(&from, &to).with_context(|| format!("Copying {from:?} to {to:?}"))?;

    // Keep filetime to avoid useless sync on some devices (e.g. Android).
    let from_metadata = from
        .metadata()
        .with_context(|| format!("Checking metadata for {from:?}"))?;
    let atime = FileTime::from_last_access_time(&from_metadata);
    let mtime = FileTime::from_last_modification_time(&from_metadata);
    set_file_times(&to, atime, mtime).with_context(|| format!("Setting times to {to:?}"))?;

    Ok(())
}

pub fn path_to_str<'a>(path: &'a Path) -> Result<&'a str> {
    Ok(path
        .to_str()
        .ok_or_else(|| anyhow!("Path is invalid '{}'", path.display()))?)
}

pub fn normalize_path(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().replace("\\", "/"))
}

pub fn contains_file_with_ext(dir_path: &Path, ext: &str) -> bool {
    if !dir_path.is_dir() {
        return false;
    };
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
    file_path.is_file()
        && file_path
            .file_name()
            .and_then(|it| it.to_str())
            .map(|it| it.ends_with(ext))
            .unwrap_or(false)
}

pub fn is_library(file_path: &Path) -> bool {
    file_path.is_file()
        && file_path
            .file_name()
            .and_then(|it| it.to_str())
            .map(|it| {
                it.ends_with(".so")
                    || it.contains(".so.")
                    || it.ends_with(".dylib")
                    || it.ends_with(".a")
            })
            .unwrap_or(false)
}

pub fn lib_name_from(file_path: &Path) -> Result<String> {
    let file_name = file_path
        .file_name()
        .and_then(|it| it.to_str())
        .ok_or_else(|| {
            anyhow!(
                "'{}' doesn't point to a valid lib name",
                file_path.display()
            )
        })?;

    let (start_index, end_index) = file_name
        .find(".so")
        .map(|end_index| (if file_name.starts_with("lib") { 3 } else { 0 }, end_index))
        .unwrap_or((0, file_name.len()));

    if start_index == end_index {
        bail!(
            "'{}' doesn't point to a valid lib name",
            file_path.display()
        );
    } else {
        Ok(file_name[start_index..end_index].to_string())
    }
}

pub fn file_name_as_str(file_path: &Path) -> Result<&str> {
    Ok(file_path
        .file_name()
        .and_then(|it| it.to_str())
        .ok_or_else(|| anyhow!("'{}' is not a valid file name", file_path.display()))?)
}

lazy_static! {
    static ref CURRENT_VERBOSITY: AtomicI8 = AtomicI8::new(0);
}

pub fn set_current_verbosity(verbosity: i8) {
    CURRENT_VERBOSITY.store(verbosity, Ordering::SeqCst)
}
pub fn get_current_verbosity() -> i8 {
    CURRENT_VERBOSITY.load(Ordering::SeqCst)
}

pub fn user_facing_log(category: &str, message: &str, verbosity: i8) {
    use colored::Colorize;
    if verbosity <= get_current_verbosity() {
        eprintln!("{:>12} {}", category.blue().bold(), message)
    }
}

pub trait LogCommandExt {
    fn log_invocation(&mut self, verbosity: i8) -> &mut Self;
}

impl LogCommandExt for Command {
    fn log_invocation(&mut self, verbosity: i8) -> &mut Self {
        user_facing_log(
            "Running",
            &format!(
                "{}{:?}",
                if verbosity + 1 < get_current_verbosity() {
                    self.get_envs()
                        .map(|(var_name, var_value)| {
                            format!(
                                "{}={:?} ",
                                var_name.to_str().unwrap(),
                                var_value.and_then(|it| it.to_str()).unwrap_or("")
                            )
                        })
                        .fold(String::new(), |mut result, env| {
                            result.push_str(&env);
                            result
                        })
                } else {
                    String::new()
                },
                self
            ),
            verbosity,
        );
        self
    }
}
