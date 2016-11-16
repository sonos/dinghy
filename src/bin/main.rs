extern crate dinghy;

use std::thread;
use std::time::Duration;

#[macro_use] extern crate log;
extern crate env_logger;

fn main() {
    env_logger::init().unwrap();

    let dinghy = dinghy::Dinghy::default();
    thread::sleep(Duration::from_millis(100));
    let d = dinghy.devices().unwrap().pop().unwrap();
    info!("Detected {} ({})", d.name(), d.target());
}
