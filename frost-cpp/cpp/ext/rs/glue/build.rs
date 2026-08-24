fn main() {
    let manifest_dir = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());

    cxx_build::bridge("src/lib.rs")
        .include(manifest_dir.join("cpp"))
        .compile("frost-glue-cxxbridge");
}
