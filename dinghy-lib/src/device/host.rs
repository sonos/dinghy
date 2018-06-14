use dinghy_build::build_env::set_env;
use itertools::Itertools;
use platform::host::HostPlatform;
use project::Project;
use std::fmt;
use std::fmt::{ Debug, Display };
use std::fmt::Formatter;
use std::sync::Arc;
use Build;
use BuildBundle;
use Device;
use PlatformManager;
use DeviceCompatibility;
use Result;
use RunEnv;
use Runnable;

pub struct HostManager {
//    compiler: Arc<Compiler>
}

impl HostManager {
    pub fn probe(/*compiler: &Arc<Compiler>*/) -> Option<HostManager> {
        Some(HostManager {
//            compiler: compiler.clone(),
        })
    }
}

impl PlatformManager for HostManager {
    fn devices(&self) -> Result<Vec<Box<Device>>> {
        Ok(vec![Box::new(HostDevice::new(/*&self.compiler*/))])
    }
}

#[derive(Clone)]
pub struct HostDevice {
    // FIXME (just cleanup)
//    compiler: Arc<Compiler>
}

impl HostDevice {
    pub fn new(/*compiler: &Arc<Compiler>*/) -> Self {
        HostDevice {
//            compiler: compiler.clone()
        }
    }

    fn install_all_apps(&self, project: &Project, build: &Build) -> Result<Vec<BuildBundle>> {
        let root_dir = build.target_path.join("dinghy");
        let bundle_libs_path = build.target_path.clone();

        let mut build_bundles = vec![];
        for runnable in &build.runnables {
            let bundle_path = root_dir.join(&runnable.id).clone();
            let bundle_exe_path = build.target_path.join(&runnable.id);

            project.link_test_data(&runnable, &bundle_path)?;

            build_bundles.push(BuildBundle {
                id: runnable.id.clone(),
                bundle_dir: bundle_path.to_path_buf(),
                bundle_exe: bundle_exe_path.to_path_buf(),
                lib_dir: bundle_libs_path.to_path_buf(),
                root_dir: root_dir.clone(),
            });
        }
        Ok(build_bundles)
    }
}

impl Device for HostDevice {
    fn clean_app(&self, _build_bundle: &BuildBundle) -> Result<()> {
        debug!("No cleanup performed as it is not required for host platform");
        Ok(())
    }

    fn debug_app(&self, project: &Project, runnable: &Runnable, run_env: &RunEnv, args: &[&str], envs: &[&str]) -> Result<()> {
        unimplemented!()
    }

    fn id(&self) -> &str {
        "HOST"
    }

    fn name(&self) -> &str {
        "host device"
    }

    fn run_app(&self, project: &Project, runnable: &Runnable, run_env: &RunEnv, args: &[&str], envs: &[&str]) -> Result<()> {
        info!("Run {} ({:?})", runnable.id, run_env.compile_mode);

        let mut cmd = ::std::process::Command::new(&runnable.exe);
        for (env_key, env_value) in envs.iter().tuples() {
            cmd.env(env_key, env_value);
        }
        cmd.args(args);
        let status = cmd.status()?;
        if !status.success() {
            Err(status)?
        }

        Ok(())
    }

    fn start_remote_lldb(&self) -> Result<String> {
        unimplemented!()
    }
}

impl Debug for HostDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        Ok(fmt.write_str(format!("Host {{ }}").as_str())?)
    }
}

impl Display for HostDevice {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.write_str("Host")
    }
}

impl DeviceCompatibility for HostDevice {
    fn is_compatible_with_host_platform(&self, _platform: &HostPlatform) -> bool {
        true
    }
}
