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
        error!("{:?}", e);
    }
}

fn run(matches: clap::ArgMatches) -> Result<()> {
    match matches.subcommand() {
        ("test", matches) => {
            let dinghy = dinghy::Dinghy::default();
            thread::sleep(Duration::from_millis(100));
            let d = dinghy.devices().unwrap().pop().unwrap();
            info!("Detected: `{}' ({})", d.name(), d.target());
            Ok(())
        },
        ("build", Some(matches)) => {
            dinghy::build::compile_tests(matches.value_of("arch").unwrap())
        }
        (sub, _) => Err(format!("Unknown subcommand {}", sub))?,
    }
}
