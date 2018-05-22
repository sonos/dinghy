use compiler::Compiler;
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

pub struct HostManager {
    compiler: Arc<Compiler>
}

impl HostManager {
    pub fn probe(compiler: &Arc<Compiler>) -> Option<HostManager> {
        Some(HostManager {
            compiler: compiler.clone(),
        })
    }
}

impl PlatformManager for HostManager {
    fn devices(&self) -> Result<Vec<Box<Device>>> {
        Ok(vec![Box::new(HostDevice::new(&self.compiler))])
    }
}


pub struct HostDevice {
    compiler: Arc<Compiler>
}

impl HostDevice {
    pub fn new(compiler: &Arc<Compiler>) -> Self {
        HostDevice {
            compiler: compiler.clone()
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

    fn debug_app(&self, _project: &Project, _build: &Build, _args: &[&str], _envs: &[&str]) -> Result<BuildBundle> {
        unimplemented!()
    }

    fn id(&self) -> &str {
        "HOST"
    }

    fn name(&self) -> &str {
        "host device"
    }

    fn run_app(&self, project: &Project, build: &Build, args: &[&str], envs: &[&str]) -> Result<Vec<BuildBundle>> {
        for (env_key, env_value) in envs.iter().tuples() {
            set_env(env_key, env_value);
        }
        let build_bundles = self.install_all_apps(project, build)?;
        self.compiler.run(None, &build.build_args, args)?;
        Ok(build_bundles)
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
