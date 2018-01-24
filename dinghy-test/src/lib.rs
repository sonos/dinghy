use std::env;
use std::path::PathBuf;

pub fn test_file_path(file_name: &str) -> PathBuf {
    if env::var("DINGHY").is_ok() {
        let current_exe = env::current_exe()
            .expect("Current exe path not accessible");

        current_exe.parent()
            .expect(format!("Current exe directory not accessible {}", current_exe.display()))
            .join(file_name)
    } else {
        path::PathBuf::from("../resources").join(file_name)
    }
}
