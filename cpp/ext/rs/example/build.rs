use frost_glue_build::{ExtensionBuilder, Type};

fn main() {
    let glue_include = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../glue/cpp");

    let mut ext = ExtensionBuilder::new("example", glue_include);

    // Primitives: each type as param and return
    ext.function("add_one")
        .param("value", &[Type::Int]);
    ext.function("negate_float")
        .param("value", &[Type::Float]);
    ext.function("flip")
        .param("value", &[Type::Bool]);
    ext.function("shout")
        .param("value", &[Type::String]);

    // Multi-type param: accepts Int or Float
    ext.function("to_float")
        .param("value", &[Type::Int, Type::Float]);

    // Any param: identity
    ext.function("identity")
        .any("value");

    // Optional param
    ext.function("greet")
        .param("name", &[Type::String])
        .optional("greeting", &[Type::String]);

    // Variadic rest
    ext.function("concat")
        .variadic_rest("parts", &[Type::String], 0);

    // Nullary
    ext.function("forty_two");

    // Error return
    ext.function("fail_if_negative")
        .param("value", &[Type::Int]);

    // Array param: sum elements
    ext.function("array_sum")
        .param("arr", &[Type::Array]);

    ext.generate();
}
