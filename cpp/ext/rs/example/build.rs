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

    // Function param: call a callback
    ext.function("apply")
        .param("func", &[Type::Function])
        .param("value", &[Type::Any]);

    // Return a Rust-created closure
    ext.function("make_adder")
        .param("n", &[Type::Int]);

    // Data-carrying functions
    ext.data_type("counter");
    ext.function("make_counter")
        .param("start", &[Type::Int]);
    ext.function("counter_value")
        .param("counter", &[Type::Function]);

    ext.generate();
}
