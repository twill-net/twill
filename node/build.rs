fn main() {
    #[cfg(feature = "std")]
    {
        // Generate build info for the node binary
        println!("cargo:rerun-if-changed=build.rs");
    }
}
