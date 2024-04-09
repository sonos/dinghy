use crate::errors::*;
use crate::project;
use crate::project::{rec_copy, Project};
use crate::utils::copy_and_sync_file;
use crate::Build;
use crate::BuildBundle;
use fs_err as fs;
use log::debug;
use std::path::Path;

pub fn make_remote_app(project: &Project, build: &Build) -> Result<BuildBundle> {
    make_remote_app_with_name(project, build, None)
}

pub fn make_remote_app_with_name(
    project: &Project,
    build: &Build,
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

    let root_dir = build.target_path.join("dinghy");
    let bundle_path = match bundle_name {
        Some(name) => root_dir.join(&build.runnable.package_name).join(name),
        None => root_dir.join(&build.runnable.package_name),
    };
    let bundle_libs_path = root_dir.join("overlay");
    let bundle_target_path = &bundle_path;
    let bundle_exe_path = bundle_target_path.join(format!("_dinghy_{}", &build.runnable.id));

    debug!("Removing previous bundle {:?}", bundle_path);
    let _ = fs::remove_dir_all(&bundle_path);
    let _ = fs::remove_dir_all(&bundle_libs_path);
    let _ = fs::remove_dir_all(&bundle_target_path);

    debug!("Making bundle {:?}", bundle_path);
    fs::create_dir_all(&bundle_path)?;
    fs::create_dir_all(&bundle_libs_path)?;
    fs::create_dir_all(&bundle_target_path)?;

    debug!(
        "Copying exe {:?} to bundle {:?}",
        &build.runnable.exe, bundle_exe_path
    );
    copy_and_sync_file(&build.runnable.exe, &bundle_exe_path).with_context(|| {
        format!(
            "Couldn't copy {} to {}",
            &build.runnable.exe.display(),
            &bundle_exe_path.display()
        )
    })?;

    debug!("Copying dynamic libs to bundle");
    for src_lib_path in &build.dynamic_libraries {
        let target_lib_path = bundle_libs_path.join(
            src_lib_path
                .file_name()
                .ok_or_else(|| anyhow!("Invalid file name {:?}", src_lib_path.file_name()))?,
        );
        if !is_sysroot_library(&src_lib_path) {
            debug!(
                "Copying dynamic lib {} to {}",
                src_lib_path.display(),
                target_lib_path.display()
            );
            copy_and_sync_file(&src_lib_path, &target_lib_path).with_context(|| {
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

    for file_in_run_args in &build.files_in_run_args {
        let dst = bundle_target_path.join(
            file_in_run_args
                .file_name()
                .ok_or_else(|| anyhow!("no file name"))?,
        );
        if file_in_run_args.is_dir() {
            rec_copy(file_in_run_args, dst, true)?;
        } else {
            copy_and_sync_file(&file_in_run_args, &dst).with_context(|| {
                format!(
                    "Couldn't copy {} to {}",
                    file_in_run_args.display(),
                    &root_dir.display()
                )
            })?;
        }
    }

    debug!(
        "Copying src {} to bundle {}",
        build.runnable.source.display(),
        bundle_path.display()
    );
    project::rec_copy_excl(
        &build.runnable.source,
        &bundle_path,
        false,
        &[build.runnable.source.join("target")],
    )?;
    debug!("Copying test_data to bundle {}", bundle_path.display());
    project.copy_test_data(&bundle_path)?;

    Ok(BuildBundle {
        id: build.runnable.id.clone(),
        bundle_dir: bundle_path.to_path_buf(),
        bundle_exe: bundle_exe_path.to_path_buf(),
        lib_dir: bundle_libs_path.to_path_buf(),
        root_dir,
    })
}
