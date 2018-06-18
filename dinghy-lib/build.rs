fn main() {
    use std::io::Write;
    let mut f = std::fs::File::create(std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("host-target-triple")).unwrap();
    write!(f, "{}", std::env::var("TARGET").unwrap()).unwrap();

    #[cfg(target_os = "macos")] {
        println!("cargo:rustc-link-search=framework=/System/Library/PrivateFrameworks");
    }
}
