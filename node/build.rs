fn main() {
    // Re-run only when build.rs itself changes.
    println!("cargo:rerun-if-changed=build.rs");
}
