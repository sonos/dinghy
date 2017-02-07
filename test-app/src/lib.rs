

#[cfg(test)]
mod tests {

    mod pass {
        use std::path;

        #[cfg(any(target_os = "ios", target_os="android"))]
        pub fn src_path() -> path::PathBuf {
            ::std::env::current_exe().unwrap().parent().unwrap().join("src")
        }

        #[cfg(not(any(target_os = "ios", target_os="android")))]
        pub fn src_path() -> path::PathBuf {
            path::PathBuf::from(".")
        }

        #[test]
        fn it_finds_source_files() {
            println!("pwd: {:?}", ::std::env::current_dir());
            println!("src_path: {:?}", src_path());
            assert!(src_path().join("src/lib.rs").exists());
        }

        #[test]
        fn it_works() {
        }
    }

    mod fails {
        #[test]
        fn it_fails() {
            panic!("Failing as expected");
        }
    }
}
