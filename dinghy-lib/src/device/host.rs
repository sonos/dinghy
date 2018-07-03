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

pub static HOST_TRIPLE: &str = include_str!(concat!(env!("OUT_DIR"), "/host-target-triple"));

pub struct HostManager { }

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

    fn install_app(&self, project: &Project, runnable: &Runnable, run_env:&RunEnv) -> Result<BuildBundle> {
        /*

        let root_dir = runnable.exe.parent().ok_or("Build artefact at root")?.join("dinghy").join(runnable.exe.file_name().unwrap());
        let bundle_path = root_dir.clone();
        project.link_test_data(&runnable, &bundle_path)?;

        Ok(BuildBundle {
            id: runnable.id.clone(),
            bundle_dir: root_dir.clone(),
            bundle_exe: runnable.exe.clone(),
            lib_dir: root_dir.join("overlays"),
            root_dir: root_dir.clone(),
        })
        */
        info!("Install {} locally", runnable.id);
        ::device::make_remote_app(project, run_env, runnable)
    }
}

impl Device for HostDevice {
    fn clean_app(&self, _build_bundle: &BuildBundle) -> Result<()> {
        debug!("No cleanup performed as it is not required for host platform");
        Ok(())
    }

    fn debug_app(&self, project: &Project, runnable: &Runnable, run_env: &RunEnv) -> Result<()> {
        unimplemented!()
    }

    fn id(&self) -> &str {
        "HOST"
    }

    fn name(&self) -> &str {
        "host device"
    }

    fn run_app(&self, project: &Project, runnable: &Runnable, run_env: &RunEnv) -> Result<()> {
        let mut cmd = if run_env.bundle {
            let installed = self.install_app(project, runnable, run_env)?;
            let mut cmd = ::std::process::Command::new(installed.bundle_exe);
            cmd.current_dir(installed.bundle_dir);
            cmd
        } else {
            ::std::process::Command::new(&runnable.exe)
        };
        info!("Run {} ({:?})", runnable.id, run_env.compile_mode);
        for pair in run_env.envs.iter() {
            let mut tokens = pair.split("=");
            let k = tokens.next().ok_or("malformed saved environment")?;
            let v = tokens.next().ok_or("malformed saved environment")?;
            cmd.env(k, v);
        }
        cmd.env("DINGHY", "1");
        cmd.args(&run_env.args);
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
