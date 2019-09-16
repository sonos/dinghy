use std::fs;
use std::process::Command;
use utils::file_name_as_str;
use Result;
use Runnable;

pub mod regular_platform;

pub fn strip_runnable(runnable: &Runnable, mut command: Command) -> Result<()> {
    let exe_stripped_name = file_name_as_str(&runnable.exe)?;

    let mut stripped_runnable = runnable.clone();
    stripped_runnable.exe = runnable
        .exe
        .parent()
        .map(|it| it.join(format!("{}-stripped", exe_stripped_name)))
        .ok_or(format!(
            "{} is not a valid executable name",
            &runnable.exe.display()
        ))?;

    // Backup old runnable
    fs::copy(&runnable.exe, &stripped_runnable.exe)?;

    let command = command.arg(&stripped_runnable.exe);
    debug!("Running command {:?}", command);

    let output = command.output()?;
    if !output.status.success() {
        bail!(
            "Error while stripping {}\nError: {}",
            &stripped_runnable.exe.display(),
            String::from_utf8(output.stdout)?
        )
    }

    debug!(
        "{} unstripped size = {} and stripped size = {}",
        runnable.exe.display(),
        fs::metadata(&runnable.exe)?.len(),
        fs::metadata(&stripped_runnable.exe)?.len()
    );
    Ok(())
}
