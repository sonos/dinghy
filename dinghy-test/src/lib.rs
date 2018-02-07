use std::fs::File;
use std::env;
use std::io::prelude::*;
use std::path::PathBuf;

pub fn test_file_path(test_data_id: &str) -> PathBuf {
    let current_exe = env::current_exe()
        .expect("Current exe path not accessible");

    if env::var("DINGHY").is_ok() {
        current_exe.parent()
            .expect(&format!("Current exe directory not accessible {}", current_exe.display()))
            .parent()

            .expect(&format!("Current exe directory not accessible {}", current_exe.display()))
            .join("test_data")
            .join(test_data_id)
    } else {
        let test_data_path = current_exe
            .parent()
            .expect(&format!("Invalid exe path {}", current_exe.display()))
            .parent()
            .expect(&format!("Current exe directory not accessible {}", current_exe.display()))
            .join("dinghy")
            .join(current_exe.file_name().expect(&format!("Invalid exe name {}", current_exe.display())))
            .join("test_data");

        let mut contents = String::new();
        let test_data_cfg_path = test_data_path.join("test_data.cfg");
        let test_data_cfg = File::open(&test_data_cfg_path)
            .and_then(|mut f| { f.read_to_string(&mut contents) })
            .expect(&format!("Couldn't read file {}", test_data_cfg_path.display()));

        let test_data_path = contents.lines()
            .map(|line| line.split(":"))
            .map(|mut line| (line.next(), line.next()))
            .find(|&(id, path)| id.map(|it| it == test_data_id).unwrap_or(false))
            .map(|(_, path)| path)
            .expect(&format!("Couldn't find test_data path {} in dinghy configuration", test_data_id))
            .expect(&format!("Couldn't find test_data path {} in dinghy configuration", test_data_id));

        PathBuf::from(test_data_path)
    }
}
