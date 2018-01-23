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
    let project = project.for_runnable(runnable)?;
    error!("YYYYYYYYYYYYYYYY {:?}", &project.conf.test_data);
    let app_name = runnable.exe.file_name()
        .ok_or(format!("App should be a file in android mode '{}'", &runnable.exe.display()))?;
    let root_path = runnable.exe.parent()
        .ok_or(format!("Invalid executable file {}", &runnable.exe.display()))?
        .join("dinghy");
    let bundle_path = root_path.join(app_name);
    let libs_path = root_path.join("overlay");
    let bundle_exe_path = bundle_path.join(app_name);

    debug!("Removing previous bundle {:?}", bundle_path);
    let _ = fs::remove_dir_all(&bundle_path);
    let _ = fs::remove_dir_all(&libs_path);

    debug!("Making bundle {:?} for {:?}", bundle_path, &runnable.exe);
    fs::create_dir_all(&bundle_path)
        .chain_err(|| format!("Couldn't create {}", &bundle_path.display()))?;
    fs::create_dir_all(&libs_path)
        .chain_err(|| format!("Couldn't create {}", &libs_path.display()))?;
    debug!("Copying exe to bundle");
    fs::copy(&runnable.exe, &bundle_exe_path)
        .chain_err(|| format!("Couldn't copy {} to {}", &runnable.exe.display(), &bundle_exe_path.display()))?;

    debug!("Copying dynamic libs to bundle");
    debug!("XXXXXXXX {:?}", &libs_path);
    for dynamic_lib in &build.dynamic_libraries {
        let lib_path = libs_path.join(dynamic_lib.file_name()
            .ok_or(format!("Invalid file name '{:?}'", dynamic_lib.file_name()))?);
        trace!("Copying dynamic lib '{}' to '{}'", dynamic_lib.display(), lib_path.display());
        fs::copy(&dynamic_lib, &lib_path)
            .chain_err(|| format!("Couldn't copy {} to {}", dynamic_lib.display(), &lib_path.display()))?;
    }

    debug!("Copying src to bundle");
    project.rec_copy(&runnable.source, &bundle_path, false)?;
    debug!("Copying test_data to bundle");
    project.copy_test_data(&bundle_path)?;

    Ok(BuildBundle {
        id: app_name.to_str().ok_or(format!("Invalid file name '{:?}'", app_name))?.to_string(),
        bundle_dir: bundle_path.to_path_buf(),
        bundle_exe: bundle_exe_path.to_path_buf(),
        lib_dir: libs_path.to_path_buf(),
    })
}
