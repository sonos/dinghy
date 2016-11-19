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
        (@subcommand test =>
        )
        (@subcommand build =>
         (@arg arch: +required "target architecture")
        )
    )
        .get_matches_from(filtered_env);

    if let Err(e) = run(matches) {
        println!("{}", e);
        std::process::exit(1);
    }
}

fn run(matches: clap::ArgMatches) -> Result<()> {
    match matches.subcommand() {
        ("test", Some(matches)) => {
            let dinghy = dinghy::Dinghy::default();
            thread::sleep(Duration::from_millis(100));
            let d = dinghy.devices().unwrap().pop().ok_or("No phone found")?;
            info!("Detected: `{}' ({}) -> {}", d.name(), d.target(), d.id());
            let tests = dinghy::build::compile_tests(&*d.target())?;
            let signing = dinghy::xcode::look_for_signature_settings(d.id())?
                .pop()
                .ok_or("no signing identity found")?;
            let app_id = signing.name.split(" ").last().ok_or("no app id ?")?;
            println!("{:?}", signing);
            info!("Will use {} -> {}", signing.identity.name, app_id);
            for t in tests {
                let app =
                    dinghy::xcode::wrap_as_app(&d.target(), "debug", &*t.0, t.1, app_id)?;
                println!("app : {:?}", app);
                dinghy::xcode::sign_app(&app, &signing)?;
                dinghy::ios::install_app(unsafe { std::mem::transmute(d.ptr()) } , app)?;
            }
            Ok(())
        }
        ("build", Some(matches)) => Ok(()),
        (sub, _) => Err(format!("Unknown subcommand {}", sub))?,
    }
}
