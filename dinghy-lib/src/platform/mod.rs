pub mod regular_platform;
pub mod host;
#[cfg(target_os = "macos")]
pub mod ios;

use std::fs;
use std::process::Command;
use utils::file_name_as_str;
use Runnable;
use Result;

fn strip_runnable(runnable: &Runnable, mut command: Command) -> Result<()> {
    let exe_stripped_name = file_name_as_str(&runnable.exe)?;

    let mut unstripped_runnable = runnable.clone();
    unstripped_runnable.exe = runnable.exe.parent()
        .map(|it| it.join(format!("{}-unstripped", exe_stripped_name)))
        .ok_or(format!("{} is not a valid executable name", &runnable.exe.display()))?;

    // Backup old runnable
    fs::copy(&runnable.exe, &unstripped_runnable.exe)?;

    let command = command.arg(&runnable.exe);
    debug!("Running command {:?}", command);

    let output = command.output()?;
    if !output.status.success() {
        bail!("Error while stripping {}\nError: {}", &unstripped_runnable.exe.display(), String::from_utf8(output.stdout)?)
    }
    Ok(())
}
