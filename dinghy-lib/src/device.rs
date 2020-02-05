use errors::*;
use project;
use project::Project;
use std::fs;
use std::path::Path;
use utils::copy_and_sync_file;
use Build;
use BuildBundle;
use Runnable;

pub fn make_remote_app(
    project: &Project,
    build: &Build,
    runnable: &Runnable,
) -> Result<BuildBundle> {
    make_remote_app_with_name(project, build, runnable, None)
}

pub fn make_remote_app_with_name(
    project: &Project,
    build: &Build,
    runnable: &Runnable,
    bundle_name: Option<&str>,
) -> Result<BuildBundle> {
    fn is_sysroot_library(path: &Path) -> bool {
        path.ancestors()
            .find(|ancestor_path| ancestor_path.ends_with("sysroot/usr/lib"))
            .is_some()
            && (!path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .eq_ignore_ascii_case("libc++_shared.so")
                && !path.to_str().unwrap().contains("android"))
    }

    let project = project.for_runnable(runnable)?;
    let root_dir = build.target_path.join("dinghy");
    let bundle_path = match bundle_name {
        Some(name) => root_dir.join(&runnable.id).join(name),
        None => root_dir.join(&runnable.id),
    };
    let bundle_libs_path = root_dir.join("overlay");
    let bundle_target_path = &bundle_path;
    let bundle_exe_path = bundle_target_path.join(format!("_dinghy_{}", &runnable.id));

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

    debug!(
        "Copying exe {:?} to bundle {:?}",
        &runnable.exe, bundle_exe_path
    );
    copy_and_sync_file(&runnable.exe, &bundle_exe_path).chain_err(|| {
        format!(
            "Couldn't copy {} to {}",
            &runnable.exe.display(),
            &bundle_exe_path.display()
        )
    })?;

    debug!("Copying dynamic libs to bundle");
    for src_lib_path in &build.dynamic_libraries {
        let target_lib_path = bundle_libs_path.join(
            src_lib_path
                .file_name()
                .ok_or(format!("Invalid file name {:?}", src_lib_path.file_name()))?,
        );
        if !is_sysroot_library(&src_lib_path) {
            debug!(
                "Copying dynamic lib {} to {}",
                src_lib_path.display(),
                target_lib_path.display()
            );
            copy_and_sync_file(&src_lib_path, &target_lib_path).chain_err(|| {
                format!(
                    "Couldn't copy {} to {}",
                    src_lib_path.display(),
                    &target_lib_path.display()
                )
            })?;
        } else {
            debug!(
                "Dynamic lib {} will not be copied as it is a sysroot library",
                src_lib_path.display()
            );
        }
    }

    debug!(
        "Copying src {} to bundle {}",
        runnable.source.display(),
        bundle_path.display()
    );
    project::rec_copy_excl(
        &runnable.source,
        &bundle_path,
        false,
        &[runnable.source.join("target")],
    )?;
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
