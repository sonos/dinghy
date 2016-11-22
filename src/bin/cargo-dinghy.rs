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
    let target = matches.value_of("TARGET").map(|s| s.to_string()).unwrap_or(d.target());
    match matches.subcommand() {
        ("run", Some(_matches)) => {
            let bin = dinghy::build::compile_bin(&*target)?.pop().expect("no executable");
            let app = d.make_app(&*bin, Some(&*target))?;
            d.install_app(&app.as_ref())?;
            d.run_app(app.as_ref(), "")?;
            Ok(())
        }
        ("test", Some(_matches)) => {
            let tests = dinghy::build::compile_tests(&*target)?;
            for t in tests {
                let app = d.make_app(&t.1, Some(&*target))?;
                d.install_app(&app.as_ref())?;
                d.run_app(app.as_ref(), "")?;
            }
            Ok(())
        }
        ("bench", Some(_matches)) => {
            let tests = dinghy::build::compile_benches(&*target)?;
            for t in tests {
                let app = d.make_app(&t.1, Some(&*target))?;
                d.install_app(&app.as_ref())?;
                d.run_app(app.as_ref(), "--bench")?;
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
