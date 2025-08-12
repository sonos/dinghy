#[cfg(test)]
extern crate dinghy_test;


/// this is an example doctest with lots of code blocks that will each generate a separate test.
///
/// ```
/// assert_eq!(2 + 2, 4);
/// ```
///
/// ```
/// assert_eq!(3 + 3, 6);
/// ```
///
/// ```
/// assert_eq!(4 + 4, 8);
/// ```
///
/// ```
/// assert_eq!(5 + 5, 10);
/// ```
///
/// ```
/// assert_eq!(6 + 6, 12);
/// ```
///
/// ```
/// assert_eq!(7 + 7, 14);
/// ```

pub fn example_function() -> i32 {
    2 + 2
}

#[cfg(test)]
mod tests {
    mod pass {
        use dinghy_test::test_file_path;
        use dinghy_test::test_project_path;
        use dinghy_test::try_test_file_path;
        use std::path;

        #[test]
        fn it_finds_source_files() {
            println!("pwd: {:?}", ::std::env::current_dir());
            println!("test_project_path: {:?}", test_project_path());
            assert!(test_project_path().join("src/lib.rs").exists());
        }

        #[test]
        fn it_finds_test_data_files() {
            println!("pwd: {:?}", ::std::env::current_dir());
            let license = if let Err(_) = ::std::env::var("NOT_BUILT_WITH_DINGHY") {
                test_file_path("dinghy_source").join("LICENSE")
            } else {
                try_test_file_path("dinghy_source")
                    .unwrap_or(path::PathBuf::from("../.."))
                    .join("LICENSE")
            };
            println!("Found path: {:?}", license);
            assert!(
                license.exists(),
                "File from dinghy_source not found: {:?}",
                license
            );
            let license = if let Err(_) = ::std::env::var("NOT_BUILT_WITH_DINGHY") {
                test_file_path("dinghy_license")
            } else {
                try_test_file_path("dinghy_license").unwrap_or(path::PathBuf::from("../../LICENSE"))
            };
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

    mod feature {
        #[test]
        fn has_feature() {
            assert!(cfg!(feature = "my-feature"))
        }
    }
}
