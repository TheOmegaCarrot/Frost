use frost_glue_build::{ExtensionBuilder, Type};

fn main() {
    let glue_include = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../glue/cpp");

    let mut ext = ExtensionBuilder::new("ini", glue_include);

    ext.function("encode")
        .param("ini", &[Type::Map])
        .optional("options", &[Type::Map]);

    ext.function("decode")
        .param("ini", &[Type::String])
        .optional("options", &[Type::Map]);

    ext.generate();
}
