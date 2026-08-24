# Rust Extension System Architecture

## Overview

Rust extensions for Frost are built on `cxx` (C++/Rust interop) and `Corrosion`
(CMake/Cargo integration). Each extension is a separate Rust crate that compiles
to a staticlib, plus a generated C++ shim that registers it with the Frost
runtime.

## Crate Layout

```
cpp/ext/rs/
├── glue/                  # frost-glue: runtime bindings to Value
│   ├── src/lib.rs         #   Safe FrostValue/FrostFunction/FrostRef API
│   ├── cpp/frost-glue.hpp #   C++ declarations (no frost headers -- cxx-build sees this)
│   ├── cpp/frost-glue.cpp #   C++ implementations (compiled by CMake, full frost access)
│   ├── build.rs           #   Runs cxx-build to generate bridge code
│   └── Cargo.toml         #   [staticlib, rlib] -- rlib for Rust deps, staticlib for Corrosion
├── glue-build/            # frost-glue-build: build-time codegen helpers
│   ├── src/lib.rs         #   ExtensionBuilder, generates C++ shims + Rust bridge
│   └── Cargo.toml         #   Used as [build-dependency] by extensions
├── example/               # Example extension exercising all patterns
│   ├── src/lib.rs         #   Extension functions
│   ├── build.rs           #   Declares functions/data types via ExtensionBuilder
│   ├── generated/         #   Build output: .cpp shim + .hpp for data types
│   └── CMakeLists.txt     #   Corrosion import + shim compilation
├── CMakeLists.txt         #   frost_rust_extension() macro, cxxbridge include path
└── get-corrosion.cmake    #   CPM fetch of Corrosion
```

## How a Rust Extension Function Gets Called

```
Frost script: ext.example.add_one(41)
        |
        v
1. Frost runtime looks up "add_one" in the ext.example module map
   (registered via REGISTER_EXTENSION in the generated .cpp)
        |
        v
2. C++ BUILTIN(add_one) -- in generated example.cpp
   - REQUIRE_ARGS validates arity and types (still on C++ side)
   - Passes the full args span as rust::Slice<const Value_Ptr>
   - Wraps call in try/catch for rust::Error -> Frost_Recoverable_Error
        |
        v
3. cxx bridge: rust_example_add_one(args: &[SharedPtr<Value>])
   (generated in bridge.rs, linked from the Rust staticlib)
        |
        v
4. Generated Rust wrapper (in bridge.rs):
   - Wraps each SharedPtr in FrostValue
   - Extracts typed values (as_int, as_string, etc.)
   - Calls the user's Rust function with native types
        |
        v
5. User's function: add_one(value: i64) -> Result<FrostValue, String>
   - Does the actual work
   - Returns Ok(FrostValue) or Err(message)
        |
        v
6. Generated wrapper converts FrostValue back to SharedPtr<Value>
   Result::Err becomes a cxx exception -> caught by C++ -> Frost_Recoverable_Error
```

## The SharedPtr Model

`Value_Ptr` (C++) = `shared_ptr<const Value>` = `SharedPtr<ffi::Value>` (Rust)

These are all the same type. Values are immutable and refcounted. When Rust
receives a SharedPtr from C++ (e.g. from the args slice), it increments the
refcount. When it drops, the refcount decrements. No copying of value data.

The `ffi::Value` type is declared once in the glue crate's bridge. Extension
crates reference it via `type Value = frost_glue::ffi::Value;` -- this tells
cxx to share SharedPtr support symbols rather than generating duplicates.

## Exception Handling

C++ exceptions cannot cross Rust frames (instant abort). The system prevents
this at every boundary:

**C++ -> Rust (calling an extension function):**
- C++ shim wraps the call in try/catch
- If Rust returns Err(String), cxx throws rust::Error
- C++ catches rust::Error, re-throws as Frost_Recoverable_Error

**Rust -> C++ (calling a Frost function from Rust):**
- value_call is declared with `-> Result<SharedPtr<Value>>` in the bridge
- cxx catches std::exception (which Frost_Recoverable_Error inherits from)
- Rust receives Err(cxx::Exception)

**Nested (Rust closure called by Frost that calls another Frost function):**
```
Frost -> C++ Builtin -> rust_closure_call -> Rust closure
         -> FrostFunction::call -> value_call -> C++ Callable::call
            -> may throw Frost_Recoverable_Error
            -> caught by cxx -> Err(cxx::Exception) in Rust
            -> Rust returns Err(String)
            -> cxx throws rust::Error
            -> C++ Builtin catches rust::Error -> Frost_Recoverable_Error
            -> Frost's try_call catches it
```

Every boundary catches. No exception ever unwinds through Rust.

## Data_Builtin (Typed Data Attached to Functions)

Some extensions need "tagged" functions -- functions that carry typed data
accessible via dynamic_cast. For example, msgpack uses this for binary blobs
and special floats.

The Rust extension system supports this via `ext.data_type("name")` in build.rs:

1. Codegen creates a C++ `Data_Builtin<DataPayload_name>` wrapper
2. The payload holds a `shared_ptr<rust::Box<RustData_name>>`
3. RustData_name stores a `Box<dyn Any>` (the user's Rust type) + a call closure
4. `try_downcast_name()` uses dynamic_cast to recover the payload, then
   Rust's Any::downcast_ref to recover the typed data
5. Multiple data types per extension are distinct C++ types -- dynamic_cast
   naturally distinguishes them

## Build System

**Corrosion** bridges CMake and Cargo. It runs `cargo build` during the CMake
build phase, producing staticlibs that CMake links.

**Key targets:**
- `cargo-build_frost_glue` -- builds the glue crate (produces rlib for deps + unused staticlib)
- `cargo-build_frost_example` -- builds the extension (staticlib includes glue)
- `frost-rs-glue` -- CMake library: frost-glue.cpp (C++ glue implementations)
- `frost-example-shim` -- CMake library: generated .cpp (BUILTIN shims + registration)

**Why the glue staticlib isn't linked directly:**
Rust staticlibs are self-contained -- they include ALL dependencies. So
`libfrost_example.a` already contains all of `frost-glue`. Linking both
`libfrost_glue.a` and `libfrost_example.a` would cause duplicate symbols.
Solution: only link extension staticlibs. The glue's Rust code comes through them.

**Build ordering:**
`frost-rs-glue` depends on `cargo-build_frost_glue` to ensure rust/cxx.h
(generated by cxx-build) exists before CMake compiles frost-glue.cpp.

## Adding a New Extension

1. Create `cpp/ext/rs/myext/` with Cargo.toml, build.rs, src/lib.rs
2. In build.rs: use ExtensionBuilder to declare functions and data types
3. In src/lib.rs: implement the functions, `include!` the generated bridge
4. In CMakeLists.txt: corrosion_import_crate + frost-myext-shim library
5. In rs/CMakeLists.txt: `frost_rust_extension(myext)`
6. In ext.cpp: `#ifdef FROST_ENABLE_MYEXT` + `DO_REGISTER_EXTENSION(myext);`
