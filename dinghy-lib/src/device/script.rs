use std::{ fs, fmt, process };
use crate::*;
use crate::config::ScriptDeviceConfiguration;

#[derive(Debug)]
pub struct ScriptDevice {
    id: String,
    conf: ScriptDeviceConfiguration,
}

impl ScriptDevice {
    fn command(&self) -> Result<process::Command> {
        if fs::metadata(&self.conf.path).is_err() {
            bail!("Can not read {:?} for {}.", self.conf.path, self.id);
        }
        let mut cmd = process::Command::new(&self.conf.path);
        cmd.arg(&self.id);
        Ok(cmd)
    }
}

impl Device for ScriptDevice {
    fn clean_app(&self, build_bundle: &BuildBundle) -> Result<()> {
        let status = self.command()?
            .arg("clean")
            .arg(&build_bundle.bundle_exe)
            .status()?;
        if !status.success() {
            Err("clean fail.")?
        }
        Ok(())
    }

    fn debug_app(&self, _project: &Project, _build: &Build, _args: &[&str], _envs: &[&str]) -> Result<BuildBundle> {
        unimplemented!()
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.id
    }

    fn run_app(&self, project: &Project, build: &Build, args: &[&str], envs: &[&str]) -> Result<Vec<BuildBundle>> {
        let root_dir = build.target_path.join("dinghy");
        let mut build_bundles = vec![];
        let args:Vec<String> = args.iter().map(|&a| ::shell_escape::escape(a.into()).to_string()).collect();
        for runnable in &build.runnables {
            let bundle_path = root_dir.join(&runnable.id).clone();
            let bundle_exe_path = build.target_path.join(&runnable.id);

            project.link_test_data(&runnable, &bundle_path)?;
            trace!("About to start runner script...");
            let status = self.command()?
                .arg("run")
                .arg(&bundle_exe_path)
                .args(&args)
//                .env(envs.iter().tuples())
                .status()?;
            if !status.success() {
                Err("Test failed")?
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
        self.conf.platform.as_ref().map_or(false, |it| *it == platform.id)
    }
}

impl Display for ScriptDevice {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self.id)
    }
}

pub struct ScriptDeviceManager {
    conf: Arc<Configuration>
}

impl ScriptDeviceManager {
    pub fn probe(conf: Arc<Configuration>) -> Option<ScriptDeviceManager> {
        Some(ScriptDeviceManager { conf })
    }
}

impl PlatformManager for ScriptDeviceManager {
    fn devices(&self) -> Result<Vec<Box<Device>>> {
        Ok(self.conf.script_devices
            .iter()
            .map(|(k, conf)| {
                Box::new(ScriptDevice {
                    id: k.clone(),
                    conf: conf.clone(),
                }) as _
            })
            .collect())
    }
}
