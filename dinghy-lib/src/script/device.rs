use crate::config::ScriptDeviceConfiguration;
use crate::*;
use std::{fmt, fs, process};

#[derive(Debug)]
pub struct ScriptDevice {
    pub id: String,
    pub conf: ScriptDeviceConfiguration,
}

impl ScriptDevice {
    fn command(&self, build: &Build) -> Result<process::Command> {
        if fs::metadata(&self.conf.path).is_err() {
            bail!("Can not read {:?} for {}.", self.conf.path, self.id);
        }
        let mut cmd = process::Command::new(&self.conf.path);
        cmd.env("DINGHY_TEST_DATA", &*self.id);
        cmd.env("DINGHY_DEVICE", &*self.id);
        if let Some(ref pf) = self.conf.platform {
            cmd.env("DINGHY_PLATFORM", &*pf);
        }
        cmd.env(
            "DINGHY_COMPILE_MODE",
            &*format!("{:?}", build.build_args.compile_mode),
        );
        Ok(cmd)
    }
}

impl Device for ScriptDevice {
    fn clean_app(&self, _build_bundle: &BuildBundle) -> Result<()> {
        Ok(())
    }

    fn debug_app(
        &self,
        _project: &Project,
        _build: &Build,
        _args: &[&str],
        _envs: &[&str],
    ) -> Result<BuildBundle> {
        unimplemented!()
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.id
    }

    fn run_app(
        &self,
        project: &Project,
        build: &Build,
        args: &[&str],
        envs: &[&str],
    ) -> Result<Vec<BuildBundle>> {
        let root_dir = build.target_path.join("dinghy");
        let mut build_bundles = vec![];
        for runnable in &build.runnables {
            let bundle_path = &runnable.source;
            let bundle_exe_path = build.target_path.join(&runnable.id);

            trace!("About to start runner script...");
            let test_data_path = project.link_test_data(&runnable, &bundle_path)?;

            let status = self
                .command(build)?
                .arg(&bundle_exe_path)
                .current_dir(&runnable.source)
                .env("DINGHY_TEST_DATA_PATH", test_data_path)
                .args(args)
                .envs(
                    envs.iter()
                        .map(|kv| {
                            Ok((
                                kv.split("=")
                                    .nth(0)
                                    .ok_or_else(|| anyhow!("Wrong env spec"))?,
                                kv.split("=")
                                    .nth(1)
                                    .ok_or_else(|| anyhow!("Wrong env spec"))?,
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?,
                )
                .status()?;
            if !status.success() {
                bail!("Test failed")
            }

            build_bundles.push(BuildBundle {
                id: runnable.id.clone(),
                bundle_dir: bundle_path.to_path_buf(),
                bundle_exe: bundle_exe_path,
                lib_dir: build.target_path.clone(),
                root_dir: root_dir.clone(),
            });
        }
        Ok(build_bundles)
    }

    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
    }
}

impl DeviceCompatibility for ScriptDevice {
    fn is_compatible_with_regular_platform(&self, platform: &RegularPlatform) -> bool {
        self.conf
            .platform
            .as_ref()
            .map_or(false, |it| *it == platform.id)
    }
}

impl Display for ScriptDevice {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.id)
    }
}
