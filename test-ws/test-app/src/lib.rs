#[cfg(test)]
extern crate dinghy_test;

#[cfg(test)]
mod tests {
    mod pass {
        use dinghy_test::test_file_path;
        use std::path;

        pub fn src_path() -> path::PathBuf {
            if cfg!(any(target_os = "ios", target_os = "android"))
                || ::std::env::var("DINGHY").is_ok()
                {
                    ::std::env::current_exe().unwrap()
                        .parent().unwrap()
                        .parent().unwrap()
                        .into()
                } else {
                path::PathBuf::from(".")
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
            let license = test_file_path("dinghy_source")
                .join("LICENSE");
            println!("Found path: {:?}", license);
            assert!(
                license.exists(),
                "File from dinghy_source not found: {:?}",
                license
            );
            let license = test_file_path("dinghy_license");
            println!("Found path: {:?}", license);
            assert!(
                license.exists(),
                "File dinghy_license not found: {:?}",
                license
            );
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
