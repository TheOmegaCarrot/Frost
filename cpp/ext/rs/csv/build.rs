use frost_glue_build::{ExtensionBuilder, Type};

fn main() {
    let glue_include = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../glue/cpp");

    let mut ext = ExtensionBuilder::new("csv", glue_include);

    ext.function("read_file")
        .param("path", &[Type::String])
        .param("options", &[Type::Map])
        .optional("callback", &[Type::Function]);

    ext.function("read_str")
        .param("path", &[Type::String])
        .param("options", &[Type::Map])
        .optional("callback", &[Type::Function]);

    ext.generate();
}
