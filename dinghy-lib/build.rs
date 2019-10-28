#[cfg(target_os = "macos")]
fn main() {
    println!("cargo:rustc-link-search=framework=/System/Library/PrivateFrameworks");
    println!("cargo:rustc-link-search=framework=/Library/Apple/System/Library/PrivateFrameworks");
}

#[cfg(not(target_os = "macos"))]
fn main() {}
