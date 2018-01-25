use errors::*;
use project::copy_file;
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
    let root_dir = build.target_path.join("dinghy");
    let bundle_path = root_dir.join(&runnable.id);
    let bundle_libs_path = root_dir.join("overlay");
    let bundle_target_path = bundle_path.join("target");
    let bundle_exe_path = bundle_target_path.join(&runnable.id);

    debug!("Removing previous bundle {:?}", bundle_path);
    let _ = fs::remove_dir_all(&bundle_path);
    let _ = fs::remove_dir_all(&bundle_libs_path);
    let _ = fs::remove_dir_all(&bundle_target_path);

    debug!("Making bundle {:?}", bundle_path);
    fs::create_dir_all(&bundle_path)
        .chain_err(|| format!("Couldn't create {}", &bundle_path.display()))?;
    fs::create_dir_all(&bundle_libs_path)
        .chain_err(|| format!("Couldn't create {}", &bundle_libs_path.display()))?;
    fs::create_dir_all(&bundle_target_path)
        .chain_err(|| format!("Couldn't create {}", &bundle_target_path.display()))?;

    debug!("Copying exe {:?} to bundle {:?}", &runnable.exe, bundle_path);
    copy_file(&runnable.exe, &bundle_exe_path)
        .chain_err(|| format!("Couldn't copy {} to {}", &runnable.exe.display(), &bundle_exe_path.display()))?;

    debug!("Copying dynamic libs to bundle");
    for src_lib_path in &build.dynamic_libraries {
        let target_lib_path = bundle_libs_path.join(src_lib_path.file_name()
            .ok_or(format!("Invalid file name {:?}", src_lib_path.file_name()))?);
        debug!("Copying dynamic lib {} to {}", src_lib_path.display(), target_lib_path.display());
        copy_file(&src_lib_path, &target_lib_path)
            .chain_err(|| format!("Couldn't copy {} to {}", src_lib_path.display(), &target_lib_path.display()))?;
    }

    debug!("Copying src {} to bundle {}", runnable.source.display(), bundle_path.display());
    project.rec_copy(&runnable.source, &bundle_path, false)?;
    debug!("Copying test_data to bundle {}", bundle_path.display());
    project.copy_test_data(&bundle_path)?;

    Ok(BuildBundle {
        id: runnable.id.clone(),
        bundle_dir: bundle_path.to_path_buf(),
        bundle_exe: bundle_exe_path.to_path_buf(),
        lib_dir: bundle_libs_path.to_path_buf(),
        root_dir: root_dir,
    })
}
