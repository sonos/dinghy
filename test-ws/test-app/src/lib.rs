

#[cfg(test)]
mod tests {

    mod pass {
        use std::path;

        pub fn src_path() -> path::PathBuf {
            if cfg!(any(target_os = "ios", target_os = "android")) ||
               ::std::env::var("DINGHY").is_ok() {
                ::std::env::current_exe().unwrap().parent().unwrap().join("src")
            } else {
                path::PathBuf::from(".")
            }
        }

        pub fn test_data_path() -> Option<path::PathBuf> {
            if cfg!(any(target_os = "ios", target_os = "android")) ||
               ::std::env::var("DINGHY").is_ok() {
                Some(::std::env::current_exe().unwrap().parent().unwrap().join("test_data"))
            } else {
                None
            }
        }

        #[test]
        fn it_finds_source_files() {
            println!("pwd: {:?}", ::std::env::current_dir());
            println!("src_path: {:?}", src_path());
            assert!(src_path().join("src/lib.rs").exists());
        }

        #[test]
        fn it_finds_test_data_files() {
            println!("pwd: {:?}", ::std::env::current_dir());
            println!("test_data path: {:?}", test_data_path());
            let license = test_data_path()
                .map(|p| p.join("dinghy_source"))
                .unwrap_or(path::PathBuf::from("../.."))
                .join("LICENSE");
            assert!(license.exists(), "File from dinghy_source not found: {:?}", license);
            let license = test_data_path()
                .map(|p| p.join("dinghy_license"))
                .unwrap_or(path::PathBuf::from("../../LICENSE"));
            assert!(license.exists(), "File dinghy_license not found: {:?}", license);
        }

        #[test]
        fn it_works() {}
    }

    mod fails {
        #[test]
        fn it_fails() {
            panic!("Failing as expected");
        }
    }
}
