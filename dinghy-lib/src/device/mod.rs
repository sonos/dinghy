use errors::*;
use project::Project;
use std::fs;
use Build;
use BuildBundle;
use Runnable;

pub mod android;
pub mod host;
pub mod ssh;

fn make_app(project: &Project, build: &Build, runnable: &Runnable) -> Result<BuildBundle> {
    let app_name = runnable.exe.file_name()
        .ok_or(format!("App should be a file in android mode '{}'", &runnable.exe.display()))?;
    let bundle_path = runnable.exe.parent()
        .ok_or(format!("Invalid executable file {}", &runnable.exe.display()))?
        .join("dinghy")
        .join(app_name);
    let bundle_exe_path = bundle_path.join(app_name);

    debug!("Removing previous bundle {:?}", bundle_path);
    let _ = fs::remove_dir_all(&bundle_path);

    debug!("Making bundle {:?} for {:?}", bundle_path, &runnable.exe);
    fs::create_dir_all(&bundle_path)
        .chain_err(|| format!("Couldn't create {}", &bundle_path.display()))?;
    debug!("Copying exe to bundle");
    fs::copy(&runnable.exe, &bundle_exe_path)
        .chain_err(|| format!("Couldn't copy {} to {}", &runnable.exe.display(), &bundle_exe_path.display()))?;

    debug!("Copying dynamic libs to bundle");
    for dynamic_lib in &build.dynamic_libraries {
        let lib_path = bundle_path.join(dynamic_lib.file_name()
            .ok_or(format!("Invalid file name '{:?}'", dynamic_lib.file_name()))?);
        trace!("Copying dynamic lib '{}'", lib_path.display());
        fs::copy(&dynamic_lib, &lib_path)
            .chain_err(|| format!("Couldn't copy {} to {}", dynamic_lib.display(), &lib_path.display()))?;
    }

    debug!("Copying src to bundle");
    project.rec_copy(&runnable.source, &bundle_path, false)?;
    debug!("Copying test_data to bundle");
    project.copy_test_data(&bundle_path)?;

    Ok(BuildBundle {
        id: app_name.to_str().ok_or(format!("Invalid file name '{:?}'", app_name))?.to_string(),
        host_dir: bundle_path.to_path_buf(),
        host_exe: bundle_exe_path.to_path_buf(),
    })
}
