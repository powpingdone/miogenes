fn main() {
    #[cfg(target_os = "android")]
    println!("cargo:rustc-link-lib=c++_shared");
}
