use std::fmt::Write;
use std::path::PathBuf;

/// A Frost type that a parameter can accept.
#[derive(Clone, Copy)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Array,
    Map,
    Function,
    /// Accepts any type.
    Any,
}

impl Type {
    fn cpp_type_name(self) -> &'static str {
        match self {
            Type::Int => "Int",
            Type::Float => "Float",
            Type::Bool => "Bool",
            Type::String => "String",
            Type::Array => "Array",
            Type::Map => "Map",
            Type::Function => "Function",
            Type::Any => unreachable!("Any has no single C++ type name"),
        }
    }
}

/// A single parameter spec for REQUIRE_ARGS.
pub struct Param {
    label: &'static str,
    types: Vec<Type>,
}

impl Param {
    fn is_single_primitive(&self) -> bool {
        self.types.len() == 1
            && matches!(
                self.types[0],
                Type::Int | Type::Float | Type::Bool | Type::String
            )
    }

    fn rust_extract(&self, var: &str) -> String {
        if self.is_single_primitive() {
            match self.types[0] {
                Type::Int => format!("{var}.as_int().unwrap()"),
                Type::Float => format!("{var}.as_float().unwrap()"),
                Type::Bool => format!("{var}.as_bool().unwrap()"),
                Type::String => format!("{var}.as_string().unwrap()"),
                _ => unreachable!(),
            }
        } else {
            var.to_owned()
        }
    }

    fn rust_extract_optional(&self, var: &str) -> String {
        if self.is_single_primitive() {
            match self.types[0] {
                Type::Int => format!("{var}.as_ref().and_then(|v| v.as_int())"),
                Type::Float => format!("{var}.as_ref().and_then(|v| v.as_float())"),
                Type::Bool => format!("{var}.as_ref().and_then(|v| v.as_bool())"),
                Type::String => format!("{var}.as_ref().and_then(|v| v.as_string())"),
                _ => unreachable!(),
            }
        } else {
            var.to_owned()
        }
    }
}

/// Whether a parameter is required, optional, or variadic rest.
pub enum ParamKind {
    Required(Param),
    Optional(Param),
    VariadicRest {
        label: &'static str,
        types: Vec<Type>,
        min_count: usize,
    },
}

/// A function to be exported from a Rust extension.
pub struct FunctionSpec {
    name: &'static str,
    params: Vec<ParamKind>,
}

impl FunctionSpec {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            params: Vec::new(),
        }
    }

    /// Add a required parameter that accepts one or more types.
    pub fn param(&mut self, label: &'static str, types: &[Type]) -> &mut Self {
        self.params.push(ParamKind::Required(Param {
            label,
            types: types.to_vec(),
        }));
        self
    }

    /// Add an optional parameter.
    pub fn optional(&mut self, label: &'static str, types: &[Type]) -> &mut Self {
        self.params.push(ParamKind::Optional(Param {
            label,
            types: types.to_vec(),
        }));
        self
    }

    /// Add a variadic rest parameter (must be last).
    pub fn variadic_rest(
        &mut self,
        label: &'static str,
        types: &[Type],
        min_count: usize,
    ) -> &mut Self {
        self.params.push(ParamKind::VariadicRest {
            label,
            types: types.to_vec(),
            min_count,
        });
        self
    }

    /// Add a required parameter that accepts any type.
    pub fn any(&mut self, label: &'static str) -> &mut Self {
        self.params.push(ParamKind::Required(Param {
            label,
            types: vec![Type::Any],
        }));
        self
    }
}

/// Builder for a Rust-backed Frost extension.
///
/// Generates two artifacts:
/// - A C++ file with BUILTIN shims and REGISTER_EXTENSION (compiled by CMake)
/// - A Rust bridge file with the cxx bridge and wrapper functions (included by
///   the extension crate via `include!`)
pub struct ExtensionBuilder {
    name: &'static str,
    functions: Vec<FunctionSpec>,
    data_types: Vec<&'static str>,
    glue_include_dir: PathBuf,
}

impl ExtensionBuilder {
    pub fn new(name: &'static str, glue_include_dir: PathBuf) -> Self {
        Self {
            name,
            functions: Vec::new(),
            data_types: Vec::new(),
            glue_include_dir,
        }
    }

    /// Declare a data type that can be attached to Function values.
    /// The name must match a Rust struct in the extension crate.
    pub fn data_type(&mut self, name: &'static str) -> &mut Self {
        self.data_types.push(name);
        self
    }

    /// Declare an exported function. Returns a mutable reference for adding
    /// params.
    pub fn function(&mut self, name: &'static str) -> &mut FunctionSpec {
        self.functions.push(FunctionSpec::new(name));
        self.functions.last_mut().unwrap()
    }

    /// Generate all artifacts and run cxx-build.
    ///
    /// The generated C++ shim is written to `generated/` under the crate's
    /// CARGO_MANIFEST_DIR so CMake can reference it at a deterministic path.
    pub fn generate(&self) {
        let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let cpp_dir = manifest_dir.join("generated");
        std::fs::create_dir_all(&cpp_dir).unwrap();

        let cpp = self.generate_cpp();
        let cpp_path = cpp_dir.join(format!("{}.cpp", self.name));
        std::fs::write(&cpp_path, &cpp).unwrap();

        // Header with data type declarations (visible to cxx-build)
        if !self.data_types.is_empty() {
            let header = self.generate_data_header();
            let header_path = cpp_dir.join(format!("{}-data.hpp", self.name));
            std::fs::write(&header_path, &header).unwrap();
        }

        let bridge_rs = self.generate_bridge_rs();
        let bridge_path = out_dir.join("bridge.rs");
        std::fs::write(&bridge_path, &bridge_rs).unwrap();

        cxx_build::bridge(&bridge_path)
            .include(&self.glue_include_dir)
            .include(&cpp_dir)
            .compile(&format!("{}-cxxbridge", self.name));

        println!("cargo:rerun-if-changed=build.rs");
    }

    // -- C++ codegen --

    fn generate_cpp(&self) -> String {
        let mut out = String::new();
        let ext = self.name;

        writeln!(out, "// Generated by frost-glue-build -- do not edit").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "#include <frost/extensions-common.hpp>").unwrap();
        writeln!(out, "#include <frost/data-builtin.hpp>").unwrap();
        writeln!(out, "#include \"frost-glue.hpp\"").unwrap();
        writeln!(out, "#include <rust/cxx.h>").unwrap();
        writeln!(out).unwrap();

        // Data type structs and forward declarations
        if !self.data_types.is_empty() {
            self.write_cpp_data_types(&mut out);
            writeln!(out).unwrap();
        }

        // Forward-declare the Rust wrapper functions.
        writeln!(out, "namespace frst::rs::{ext}").unwrap();
        writeln!(out, "{{").unwrap();
        for func in &self.functions {
            writeln!(
                out,
                "Value_Ptr rust_{ext}_{name}(::rust::Slice<const Value_Ptr> args);",
                name = func.name,
            )
            .unwrap();
        }
        writeln!(out, "}} // namespace frst::rs::{ext}").unwrap();

        writeln!(out).unwrap();
        writeln!(out, "namespace frst").unwrap();
        writeln!(out, "{{").unwrap();
        writeln!(out, "namespace {ext}").unwrap();
        writeln!(out, "{{").unwrap();
        writeln!(out, "namespace").unwrap();
        writeln!(out, "{{").unwrap();

        for func in &self.functions {
            writeln!(out).unwrap();
            self.write_cpp_builtin(&mut out, func);
        }

        writeln!(out).unwrap();
        writeln!(out, "}} // anon namespace").unwrap();
        writeln!(out, "}} // namespace {ext}").unwrap();
        writeln!(out).unwrap();

        write!(out, "REGISTER_EXTENSION({ext}").unwrap();
        for func in &self.functions {
            write!(out, ", ENTRY({})", func.name).unwrap();
        }
        writeln!(out, ")").unwrap();

        writeln!(out).unwrap();
        writeln!(out, "}} // namespace frst").unwrap();

        out
    }

    fn write_cpp_builtin(&self, out: &mut String, func: &FunctionSpec) {
        let ext = self.name;
        let name = func.name;
        let qualified = format!("{ext}.{name}");

        writeln!(out, "BUILTIN({name})").unwrap();
        writeln!(out, "{{").unwrap();

        // Arity and type validation
        if func.params.is_empty() {
            writeln!(out, "    REQUIRE_NULLARY(\"{qualified}\");").unwrap();
        } else {
            write!(out, "    REQUIRE_ARGS(\"{qualified}\"").unwrap();
            for pk in &func.params {
                write!(out, ", ").unwrap();
                match pk {
                    ParamKind::Required(p) => {
                        write!(out, "PARAM(\"{}\", {})", p.label, types_macro(&p.types))
                            .unwrap();
                    }
                    ParamKind::Optional(p) => {
                        write!(
                            out,
                            "OPTIONAL(PARAM(\"{}\", {}))",
                            p.label,
                            types_macro(&p.types)
                        )
                        .unwrap();
                    }
                    ParamKind::VariadicRest {
                        label,
                        types,
                        min_count,
                    } => {
                        write!(
                            out,
                            "VARIADIC_REST({min_count}, \"{label}\", {})",
                            types_macro(types)
                        )
                        .unwrap();
                    }
                }
            }
            writeln!(out, ");").unwrap();
        }

        // Pass entire args span to Rust
        writeln!(out, "    try {{").unwrap();
        writeln!(
            out,
            "        return frst::rs::{ext}::rust_{ext}_{name}(\
             {{args.data(), args.size()}});"
        )
        .unwrap();
        writeln!(out, "    }} catch (const ::rust::Error& e) {{").unwrap();
        writeln!(
            out,
            "        throw Frost_Recoverable_Error{{std::string(e.what())}};"
        )
        .unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "}}").unwrap();
    }

    fn generate_data_header(&self) -> String {
        let mut out = String::new();
        let ext = self.name;

        writeln!(out, "// Generated by frost-glue-build -- do not edit").unwrap();
        writeln!(out, "#pragma once").unwrap();
        writeln!(out, "#include \"frost-glue.hpp\"").unwrap();
        writeln!(out, "#include <rust/cxx.h>").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "namespace frst::rs::{ext}").unwrap();
        writeln!(out, "{{").unwrap();

        for dt in &self.data_types {
            writeln!(out, "struct RustData_{dt};").unwrap();
            writeln!(
                out,
                "std::shared_ptr<const ::frst::rs::Value> make_data_{dt}(\
                 ::rust::Box<RustData_{dt}> data);"
            )
            .unwrap();
            writeln!(
                out,
                "RustData_{dt} const* try_downcast_{dt}(\
                 ::frst::rs::Value const& val);"
            )
            .unwrap();
        }

        writeln!(out, "}} // namespace frst::rs::{ext}").unwrap();
        out
    }

    fn write_cpp_data_types(&self, out: &mut String) {
        let ext = self.name;

        writeln!(out, "namespace frst::rs::{ext}").unwrap();
        writeln!(out, "{{").unwrap();

        // Forward-declare opaque Rust types and their call functions
        for dt in &self.data_types {
            writeln!(out, "struct RustData_{dt};").unwrap();
            writeln!(
                out,
                "Value_Ptr rust_data_{dt}_call(\
                 RustData_{dt} const& data, \
                 ::rust::Slice<const Value_Ptr> args);"
            )
            .unwrap();
        }
        writeln!(out).unwrap();

        // Data_Builtin payload structs (shared_ptr for dual ownership)
        for dt in &self.data_types {
            writeln!(out, "struct DataPayload_{dt}").unwrap();
            writeln!(out, "{{").unwrap();
            writeln!(
                out,
                "    std::shared_ptr<::rust::Box<RustData_{dt}>> inner;"
            )
            .unwrap();
            writeln!(out, "}};").unwrap();
            writeln!(out).unwrap();
        }

        // make_data_<name> functions
        for dt in &self.data_types {
            writeln!(
                out,
                "Value_Ptr make_data_{dt}(::rust::Box<RustData_{dt}> data)"
            )
            .unwrap();
            writeln!(out, "{{").unwrap();
            writeln!(
                out,
                "    auto shared = std::make_shared<::rust::Box<RustData_{dt}>>(\
                 std::move(data));"
            )
            .unwrap();
            writeln!(out, "    return Value::create(").unwrap();
            writeln!(
                out,
                "        Function{{std::make_shared<Data_Builtin<DataPayload_{dt}>>("
            )
            .unwrap();
            writeln!(
                out,
                "            [shared](builtin_args_t args) -> Value_Ptr {{"
            )
            .unwrap();
            writeln!(out, "                try {{").unwrap();
            writeln!(
                out,
                "                    return rust_data_{dt}_call(\
                 **shared, {{args.data(), args.size()}});"
            )
            .unwrap();
            writeln!(
                out,
                "                }} catch (const ::rust::Error& e) {{"
            )
            .unwrap();
            writeln!(
                out,
                "                    throw Frost_Recoverable_Error{{\
                 std::string(e.what())}};"
            )
            .unwrap();
            writeln!(out, "                }}").unwrap();
            writeln!(
                out,
                "            }}, \"<rust data:{dt}>\", DataPayload_{dt}{{shared}})}});"
            )
            .unwrap();
            writeln!(out, "}}").unwrap();
            writeln!(out).unwrap();
        }

        // try_downcast_<name> functions
        for dt in &self.data_types {
            writeln!(
                out,
                "RustData_{dt} const* try_downcast_{dt}(Value const& val)"
            )
            .unwrap();
            writeln!(out, "{{").unwrap();
            writeln!(out, "    if (not val.is<Function>())").unwrap();
            writeln!(out, "        return nullptr;").unwrap();
            writeln!(
                out,
                "    auto* db = dynamic_cast<Data_Builtin<DataPayload_{dt}> const*>(\
                 val.raw_get<Function>().get());"
            )
            .unwrap();
            writeln!(out, "    if (not db)").unwrap();
            writeln!(out, "        return nullptr;").unwrap();
            writeln!(out, "    return &**db->data().inner;").unwrap();
            writeln!(out, "}}").unwrap();
            writeln!(out).unwrap();
        }

        writeln!(out, "}} // namespace frst::rs::{ext}").unwrap();
    }

    // -- Rust codegen --

    fn generate_bridge_rs(&self) -> String {
        let mut out = String::new();
        let ext = self.name;

        writeln!(out, "// Generated by frost-glue-build -- do not edit").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "#[cxx::bridge(namespace = \"frst::rs::{ext}\")]").unwrap();
        writeln!(out, "mod ffi {{").unwrap();
        writeln!(out, "    unsafe extern \"C++\" {{").unwrap();
        writeln!(out, "        include!(\"frost-glue.hpp\");").unwrap();
        if !self.data_types.is_empty() {
            writeln!(out, "        include!(\"{ext}-data.hpp\");").unwrap();
        }
        writeln!(out, "        #[namespace = \"frst::rs\"]").unwrap();
        writeln!(out, "        type Value = frost_glue::ffi::Value;").unwrap();

        // Data type: make and try_downcast functions
        for dt in &self.data_types {
            writeln!(out).unwrap();
            writeln!(
                out,
                "        fn make_data_{dt}(data: Box<RustData_{dt}>) -> SharedPtr<Value>;"
            )
            .unwrap();
            writeln!(
                out,
                "        fn try_downcast_{dt}(val: &Value) -> *const RustData_{dt};"
            )
            .unwrap();
        }

        writeln!(out, "    }}").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "    extern \"Rust\" {{").unwrap();

        for func in &self.functions {
            writeln!(
                out,
                "        fn rust_{ext}_{name}(args: &[SharedPtr<Value>]) \
                 -> Result<SharedPtr<Value>>;",
                name = func.name,
            )
            .unwrap();
        }

        // Data type call functions
        for dt in &self.data_types {
            writeln!(out, "        type RustData_{dt};").unwrap();
            writeln!(
                out,
                "        fn rust_data_{dt}_call(data: &RustData_{dt}, \
                 args: &[SharedPtr<Value>]) -> Result<SharedPtr<Value>>;"
            )
            .unwrap();
        }

        writeln!(out, "    }}").unwrap();
        writeln!(out, "}}").unwrap();

        for func in &self.functions {
            writeln!(out).unwrap();
            self.write_rust_wrapper(&mut out, func);
        }

        // Data type wrappers
        for dt in &self.data_types {
            writeln!(out).unwrap();
            self.write_rust_data_type(&mut out, dt);
        }

        out
    }

    fn write_rust_wrapper(&self, out: &mut String, func: &FunctionSpec) {
        let ext = self.name;
        let name = func.name;

        let args_name = if func.params.is_empty() { "_args" } else { "args" };
        writeln!(
            out,
            "fn rust_{ext}_{name}({args_name}: &[cxx::SharedPtr<ffi::Value>]) \
             -> Result<cxx::SharedPtr<ffi::Value>, String> {{"
        )
        .unwrap();

        // Bind each arg as a FrostValue (required) or Option<FrostValue> (optional)
        for (i, pk) in func.params.iter().enumerate() {
            match pk {
                ParamKind::Required(_) => {
                    writeln!(
                        out,
                        "    let val{i} = frost_glue::FrostValue::from_shared(args[{i}].clone());"
                    )
                    .unwrap();
                }
                ParamKind::Optional(_) => {
                    writeln!(
                        out,
                        "    let val{i} = args.get({i})\
                         .map(|v| frost_glue::FrostValue::from_shared(v.clone()));"
                    )
                    .unwrap();
                }
                ParamKind::VariadicRest { .. } => {
                    writeln!(
                        out,
                        "    let rest: Vec<frost_glue::FrostValue> = args[{i}..].iter()\
                         .map(|v| frost_glue::FrostValue::from_shared(v.clone())).collect();"
                    )
                    .unwrap();
                }
            }
        }

        // Call the user's function
        write!(out, "    let result = {name}(").unwrap();

        let mut first = true;
        for (i, pk) in func.params.iter().enumerate() {
            if !first {
                write!(out, ", ").unwrap();
            }
            first = false;
            match pk {
                ParamKind::Required(p) => {
                    write!(out, "{}", p.rust_extract(&format!("val{i}"))).unwrap();
                }
                ParamKind::Optional(p) => {
                    write!(out, "{}", p.rust_extract_optional(&format!("val{i}"))).unwrap();
                }
                ParamKind::VariadicRest { .. } => {
                    write!(out, "&rest").unwrap();
                }
            }
        }

        writeln!(out, ")?;").unwrap();
        writeln!(out, "    Ok(result.into_shared())").unwrap();
        writeln!(out, "}}").unwrap();
    }

    fn write_rust_data_type(&self, out: &mut String, dt: &str) {
        let ext = self.name;

        // The RustData_<name> struct wraps the user's type + closure.
        // It's what gets boxed and passed to C++.
        writeln!(out, "pub struct RustData_{dt} {{").unwrap();
        writeln!(out, "    data: Box<dyn std::any::Any>,").unwrap();
        writeln!(
            out,
            "    call: Box<dyn Fn(&dyn std::any::Any, &[cxx::SharedPtr<ffi::Value>]) \
             -> Result<cxx::SharedPtr<ffi::Value>, String>>,"
        )
        .unwrap();
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();

        // The call function invoked by C++ through the Data_Builtin
        writeln!(
            out,
            "fn rust_data_{dt}_call(\
             data: &RustData_{dt}, \
             args: &[cxx::SharedPtr<ffi::Value>]\
             ) -> Result<cxx::SharedPtr<ffi::Value>, String> {{"
        )
        .unwrap();
        writeln!(out, "    (data.call)(&*data.data, args)").unwrap();
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();

        // Public constructor: create a data-carrying function
        writeln!(
            out,
            "pub fn make_data_{dt}<T, F>(data: T, call: F) -> frost_glue::FrostValue"
        )
        .unwrap();
        writeln!(out, "where").unwrap();
        writeln!(out, "    T: 'static,").unwrap();
        writeln!(
            out,
            "    F: Fn(&T, &[frost_glue::FrostValue]) -> Result<frost_glue::FrostValue, String> + 'static,"
        )
        .unwrap();
        writeln!(out, "{{").unwrap();
        writeln!(out, "    let wrapper = move |any: &dyn std::any::Any, args: &[cxx::SharedPtr<ffi::Value>]| -> Result<cxx::SharedPtr<ffi::Value>, String> {{").unwrap();
        writeln!(out, "        let typed = any.downcast_ref::<T>().unwrap();").unwrap();
        writeln!(out, "        let frost_args: Vec<frost_glue::FrostValue> = args.iter()").unwrap();
        writeln!(out, "            .map(|a| frost_glue::FrostValue::from_shared(a.clone())).collect();").unwrap();
        writeln!(out, "        let result = call(typed, &frost_args)?;").unwrap();
        writeln!(out, "        Ok(result.into_shared())").unwrap();
        writeln!(out, "    }};").unwrap();
        writeln!(
            out,
            "    let rust_data = Box::new(RustData_{dt} {{ data: Box::new(data), call: Box::new(wrapper) }});"
        )
        .unwrap();
        writeln!(
            out,
            "    frost_glue::FrostValue::from_shared(ffi::make_data_{dt}(rust_data))"
        )
        .unwrap();
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();

        // Public downcast: recover the typed data from a FrostValue
        writeln!(
            out,
            "pub fn try_downcast_{dt}<T: 'static>(val: &frost_glue::FrostValue) -> Option<&T> {{"
        )
        .unwrap();
        writeln!(
            out,
            "    let ptr = ffi::try_downcast_{dt}(val.as_raw());",
        )
        .unwrap();
        writeln!(out, "    if ptr.is_null() {{").unwrap();
        writeln!(out, "        None").unwrap();
        writeln!(out, "    }} else {{").unwrap();
        writeln!(out, "        let data = unsafe {{ &*ptr }};").unwrap();
        writeln!(out, "        data.data.downcast_ref::<T>()").unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "}}").unwrap();
    }
}

fn types_macro(types: &[Type]) -> String {
    if types.len() == 1 && matches!(types[0], Type::Any) {
        return "ANY".to_owned();
    }

    let inner: Vec<&str> = types.iter().map(|t| t.cpp_type_name()).collect();
    format!("TYPES({})", inner.join(", "))
}
