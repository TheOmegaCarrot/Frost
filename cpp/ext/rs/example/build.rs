use frost_glue_build::{ExtensionBuilder, Type};

fn main() {
    let glue_include = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../glue/cpp");

    let mut ext = ExtensionBuilder::new("example", glue_include);
    ext.function("concat")
        .param("a", &[Type::String])
        .param("b", &[Type::String]);
    ext.generate();
}
