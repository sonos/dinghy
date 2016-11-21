#[macro_use]
extern crate clap;
extern crate dinghy;
extern crate env_logger;
#[macro_use]
extern crate log;

use std::thread;
use std::time::Duration;

use dinghy::errors::*;


fn main() {
    env_logger::init().unwrap();

    let filtered_env = ::std::env::args()
        .enumerate()
        .filter(|&(ix, ref s)| !(ix == 1 && s == "dinghy"))
        .map(|(_, s)| s);

    let matches = clap_app!(dinghy =>
        (@arg TARGET: --target +takes_value "target triple (rust convention)")
        (@subcommand test =>
        )
        (@subcommand run =>
        )
        (@subcommand bench =>
        )
        (@subcommand build =>
        )
        (@subcommand lldbproxy =>
        )
    )
        .get_matches_from(filtered_env);

    if let Err(e) = run(matches) {
        println!("{}", e);
        std::process::exit(1);
    }
}

fn run(matches: clap::ArgMatches) -> Result<()> {
    let dinghy = dinghy::Dinghy::default();
    thread::sleep(Duration::from_millis(100));
    let d = dinghy.devices().unwrap().pop().ok_or("No phone found")?;
    info!("Detected: `{}' ({}) -> {}", d.name(), d.target(), d.id());
    let signing = dinghy::xcode::look_for_signature_settings(d.id())?
        .pop()
        .ok_or("no signing identity found")?;
    debug!("{:?}", signing);
    let app_id = signing.name.split(" ").last().ok_or("no app id ?")?;
    info!("Will use {} -> {}", signing.identity.name, app_id);
    let target = matches.value_of("TARGET").map(|s|s.to_string()).unwrap_or(d.target());
    match matches.subcommand() {
        ("run", Some(_matches)) => {
            let bin = dinghy::build::compile_bin(&*target)?.pop().expect("no executable");
            let app =
                dinghy::xcode::wrap_as_app(&*target, "debug", "main", bin, app_id)?;
            dinghy::xcode::sign_app(&app, &signing)?;
            d.install_app(&app.as_ref())?;
            d.run_app(app.as_ref(), &app_id, "")?;
            Ok(())
        }
        ("test", Some(_matches)) => {
            let tests = dinghy::build::compile_tests(&*target)?;
            for t in tests {
                let app =
                    dinghy::xcode::wrap_as_app(&*target, "debug", &*t.0, t.1, app_id)?;
                dinghy::xcode::sign_app(&app, &signing)?;
                d.install_app(&app.as_ref())?;
                d.run_app(app.as_ref(), &app_id, "")?;
            }
            Ok(())
        }
        ("bench", Some(_matches)) => {
            let tests = dinghy::build::compile_benches(&*target)?;
            for t in tests {
                let app =
                    dinghy::xcode::wrap_as_app(&*target, "release", &*t.0, t.1, app_id)?;
                debug!("app: {:?}", app);
                dinghy::xcode::sign_app(&app, &signing)?;
                d.install_app(&app.as_ref())?;
                d.run_app(app.as_ref(), &app_id, "--bench")?;
            }
            Ok(())
        }
        ("lldbproxy", Some(_matches)) => {
            let dinghy = dinghy::Dinghy::default();
            thread::sleep(Duration::from_millis(100));
            let d = dinghy.devices().unwrap().pop().ok_or("No phone found")?;
            let lldb = d.start_remote_lldb()?;
            println!("lldb running at: {}", lldb);
            loop {
                thread::sleep(Duration::from_millis(100));
            }
        }
        ("build", Some(_matches)) => Ok(()),
        (sub, _) => Err(format!("Unknown subcommand {}", sub))?,
    }
}
