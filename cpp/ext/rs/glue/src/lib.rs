//! # frost-glue: Rust bindings to the Frost Value system
//!
//! This crate provides safe Rust access to Frost's core `Value` type, which
//! is a refcounted, immutable, dynamically-typed value used throughout the
//! Frost interpreter.
//!
//! ## Architecture
//!
//! The FFI boundary uses the `cxx` crate. The layers are:
//!
//! ```text
//! Extension Rust code (uses FrostValue, FrostFunction, FrostRef)
//!        |
//! frost-glue (this crate) -- safe wrappers
//!        |
//! ffi module -- cxx bridge declarations (raw SharedPtr<Value>)
//!        |
//! frost-glue.hpp / frost-glue.cpp -- C++ free functions
//!        |
//! Frost's Value class (cpp/value/)
//! ```
//!
//! ## Ownership model
//!
//! Every `FrostValue` holds a `cxx::SharedPtr<Value>`, which is a C++
//! `std::shared_ptr<const frst::Value>`. Values are immutable and
//! reference-counted. Cloning a `FrostValue` increments the refcount (cheap).
//! Dropping decrements it. The underlying data is freed when the last
//! reference (Rust or C++) goes away.
//!
//! ## Exception boundary
//!
//! C++ Frost functions can throw `Frost_Recoverable_Error`. The cxx bridge
//! catches these (they inherit from `std::exception`) and maps them to
//! `Result::Err(cxx::Exception)` on the Rust side.
//!
//! Going the other direction: when Rust returns `Err(String)` from an
//! `extern "Rust"` function, cxx throws `rust::Error` on the C++ side.
//! The generated C++ shims catch `rust::Error` and re-throw as
//! `Frost_Recoverable_Error`, making Rust errors catchable by Frost's
//! `try_call`.

// The cxx bridge: raw FFI declarations between Rust and C++.
// These functions are implemented in frost-glue.cpp (compiled by CMake).
// Extension crates reference the `Value` type from this module via
// `type Value = frost_glue::ffi::Value` in their own bridges.
#[cxx::bridge(namespace = "frst::rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("frost-glue.hpp");

        /// Opaque handle to a Frost Value. On the C++ side this is
        /// `const frst::Value` (typedef'd as `frst::rs::Value`).
        /// Always held behind `SharedPtr` -- never null.
        type Value;

        // ---- Factories ----
        fn value_null() -> SharedPtr<Value>;
        fn value_from_int(val: i64) -> SharedPtr<Value>;
        fn value_from_float(val: f64) -> Result<SharedPtr<Value>>;
        fn value_from_bool(val: bool) -> SharedPtr<Value>;
        fn value_from_string(val: &CxxString) -> SharedPtr<Value>;
        fn value_from_array(elements: &[SharedPtr<Value>]) -> SharedPtr<Value>;
        fn value_from_map(
            keys: &[SharedPtr<Value>],
            values: &[SharedPtr<Value>],
        ) -> Result<SharedPtr<Value>>;
        fn value_from_map_trusted(
            keys: &[SharedPtr<Value>],
            values: &[SharedPtr<Value>],
        ) -> SharedPtr<Value>;

        // ---- Type checks ----
        fn value_is_null(val: &Value) -> bool;
        fn value_is_int(val: &Value) -> bool;
        fn value_is_float(val: &Value) -> bool;
        fn value_is_bool(val: &Value) -> bool;
        fn value_is_string(val: &Value) -> bool;
        fn value_is_array(val: &Value) -> bool;
        fn value_is_map(val: &Value) -> bool;
        fn value_is_function(val: &Value) -> bool;

        // ---- Primitive accessors (unchecked -- use FrostValue wrapper) ----
        fn value_get_int(val: &Value) -> i64;
        fn value_get_float(val: &Value) -> f64;
        fn value_get_bool(val: &Value) -> bool;
        fn value_get_string(val: &Value) -> &CxxString;

        // ---- Array accessors (unchecked -- caller must verify is_array) ----
        fn value_array_len(val: &Value) -> usize;
        fn value_array_get(val: &Value, index: usize) -> SharedPtr<Value>;
        fn value_array_slice(val: &Value) -> &[SharedPtr<Value>];

        // ---- Map accessors (unchecked -- caller must verify is_map) ----
        fn value_map_len(val: &Value) -> usize;
        fn value_map_has(val: &Value, key: &CxxString) -> bool;
        fn value_map_get(val: &Value, key: &CxxString) -> SharedPtr<Value>;
        fn value_map_get_by(map: &Value, key: &SharedPtr<Value>) -> SharedPtr<Value>;
        fn value_map_contains_key(map: &Value, key: &SharedPtr<Value>) -> bool;
        fn value_map_keys(val: &Value) -> &[SharedPtr<Value>];
        fn value_map_values(val: &Value) -> &[SharedPtr<Value>];

        // ---- Function call (caller must verify is_function first) ----
        // Frost exceptions are caught by cxx and returned as Err.
        fn value_call(
            callable: &Value,
            args: &[SharedPtr<Value>],
        ) -> Result<SharedPtr<Value>>;

        // ---- Arithmetic (throw on type mismatch -> Result::Err) ----
        fn value_add(lhs: &SharedPtr<Value>, rhs: &SharedPtr<Value>) -> Result<SharedPtr<Value>>;
        fn value_subtract(lhs: &SharedPtr<Value>, rhs: &SharedPtr<Value>) -> Result<SharedPtr<Value>>;
        fn value_multiply(lhs: &SharedPtr<Value>, rhs: &SharedPtr<Value>) -> Result<SharedPtr<Value>>;
        fn value_divide(lhs: &SharedPtr<Value>, rhs: &SharedPtr<Value>) -> Result<SharedPtr<Value>>;
        fn value_modulus(lhs: &SharedPtr<Value>, rhs: &SharedPtr<Value>) -> Result<SharedPtr<Value>>;

        // ---- Comparison ----
        fn value_equal(lhs: &SharedPtr<Value>, rhs: &SharedPtr<Value>) -> Result<SharedPtr<Value>>;
        fn value_not_equal(lhs: &SharedPtr<Value>, rhs: &SharedPtr<Value>) -> Result<SharedPtr<Value>>;
        fn value_less_than(lhs: &SharedPtr<Value>, rhs: &SharedPtr<Value>) -> Result<SharedPtr<Value>>;
        fn value_less_than_or_equal(lhs: &SharedPtr<Value>, rhs: &SharedPtr<Value>) -> Result<SharedPtr<Value>>;
        fn value_greater_than(lhs: &SharedPtr<Value>, rhs: &SharedPtr<Value>) -> Result<SharedPtr<Value>>;
        fn value_greater_than_or_equal(lhs: &SharedPtr<Value>, rhs: &SharedPtr<Value>) -> Result<SharedPtr<Value>>;

        // ---- Coercion ----
        fn value_truthy(val: &Value) -> bool;

        // ---- Closure creation ----
        // Wraps a Rust closure in a C++ Builtin. When called from Frost,
        // the Builtin invokes rust_closure_call. If Rust returns Err,
        // it becomes a Frost_Recoverable_Error.
        fn make_closure(closure: Box<RustClosure>) -> SharedPtr<Value>;

        // ---- Stringification ----
        fn value_to_string(val: &Value) -> UniquePtr<CxxString>;
        fn value_type_name(val: &Value) -> UniquePtr<CxxString>;
    }

    extern "Rust" {
        /// Opaque Rust type holding a boxed closure. C++ calls through
        /// this via rust_closure_call when a Rust-created function is
        /// invoked from Frost.
        type RustClosure;

        /// Called by C++ (from within a Builtin::call) to invoke the
        /// Rust closure. Returns Err -> rust::Error -> Frost_Recoverable_Error.
        fn rust_closure_call(
            closure: &RustClosure,
            args: &[SharedPtr<Value>],
        ) -> Result<SharedPtr<Value>>;
    }
}

use cxx::let_cxx_string;

/// The function type stored inside RustClosure.
/// Takes raw SharedPtr args (already validated by REQUIRE_ARGS on the C++
/// side) and returns either a new Value or an error string.
type ClosureFn = Box<dyn Fn(&[cxx::SharedPtr<ffi::Value>]) -> Result<cxx::SharedPtr<ffi::Value>, String> + Send + Sync>;

/// Opaque type that C++ holds (via `rust::Box<RustClosure>`) inside a
/// `Data_Builtin` or plain `Builtin`. When Frost calls the function,
/// C++ invokes `rust_closure_call` which delegates to `self.f`.
pub struct RustClosure {
    f: ClosureFn,
}

/// Bridge function: C++ calls this to invoke a Rust closure.
/// The `Result::Err` path causes cxx to throw `rust::Error`, which the
/// C++ wrapper catches and re-throws as `Frost_Recoverable_Error`.
fn rust_closure_call(
    closure: &RustClosure,
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    (closure.f)(args)
}

// ===========================================================================
// FrostValue -- the main type extension authors interact with
// ===========================================================================

/// A Frost value. This is the primary type for Rust extension authors.
///
/// Wraps a C++ `shared_ptr<const Value>` (the same refcounted pointer used
/// throughout the Frost interpreter). Immutable -- all "modification"
/// creates new values.
///
/// # Creating values
/// ```ignore
/// FrostValue::null()
/// FrostValue::from_int(42)
/// FrostValue::from_float(3.14)?   // Err on NaN/Infinity
/// FrostValue::from_bool(true)
/// FrostValue::from_string("hello")
/// FrostValue::from_values(&[a, b, c])  // Array
/// ```
///
/// # Inspecting values
/// ```ignore
/// match value.unpack() {
///     FrostRef::Int(n) => ...,
///     FrostRef::String(s) => ...,
///     FrostRef::Array(elems) => ...,
///     _ => ...,
/// }
/// ```
pub struct FrostValue {
    inner: cxx::SharedPtr<ffi::Value>,
}

impl FrostValue {
    // ---- Factories ----

    /// The Frost null value (singleton).
    pub fn null() -> Self {
        Self { inner: ffi::value_null() }
    }

    /// Create a Frost Int from a Rust i64.
    pub fn from_int(val: i64) -> Self {
        Self { inner: ffi::value_from_int(val) }
    }

    /// Create a Frost Float. Returns `Err` if the value is NaN or infinity
    /// (Frost rejects these at construction time).
    pub fn from_float(val: f64) -> Result<Self, cxx::Exception> {
        Ok(Self { inner: ffi::value_from_float(val)? })
    }

    /// Create a Frost Bool.
    pub fn from_bool(val: bool) -> Self {
        Self { inner: ffi::value_from_bool(val) }
    }

    /// Create a Frost String from a UTF-8 `&str`.
    pub fn from_string(val: &str) -> Self {
        let_cxx_string!(s = val);
        Self { inner: ffi::value_from_string(&s) }
    }

    /// Create a Frost Array from a slice of raw shared pointers.
    /// Use `from_values` for the higher-level API.
    pub fn from_array(elements: &[cxx::SharedPtr<ffi::Value>]) -> Self {
        Self { inner: ffi::value_from_array(elements) }
    }

    /// Create a Frost Array from a slice of FrostValues.
    pub fn from_values(elements: &[FrostValue]) -> Self {
        let shared: Vec<cxx::SharedPtr<ffi::Value>> =
            elements.iter().map(|v| v.inner.clone()).collect();
        Self::from_array(&shared)
    }

    /// Create a Frost Map from parallel key/value slices.
    /// Keys must be non-null primitives (Int, Float, Bool, or String).
    /// Returns `Err` if key validation fails.
    pub fn from_map(
        keys: &[cxx::SharedPtr<ffi::Value>],
        values: &[cxx::SharedPtr<ffi::Value>],
    ) -> Result<Self, cxx::Exception> {
        Ok(Self { inner: ffi::value_from_map(keys, values)? })
    }

    /// Create a Frost Map without key validation.
    /// Caller must guarantee all keys are non-null primitives.
    pub fn from_map_trusted(
        keys: &[cxx::SharedPtr<ffi::Value>],
        values: &[cxx::SharedPtr<ffi::Value>],
    ) -> Self {
        Self { inner: ffi::value_from_map_trusted(keys, values) }
    }

    // ---- Type checks ----

    pub fn is_null(&self) -> bool {
        ffi::value_is_null(&self.inner)
    }

    pub fn is_int(&self) -> bool {
        ffi::value_is_int(&self.inner)
    }

    pub fn is_float(&self) -> bool {
        ffi::value_is_float(&self.inner)
    }

    pub fn is_bool(&self) -> bool {
        ffi::value_is_bool(&self.inner)
    }

    pub fn is_string(&self) -> bool {
        ffi::value_is_string(&self.inner)
    }

    pub fn is_array(&self) -> bool {
        ffi::value_is_array(&self.inner)
    }

    pub fn is_map(&self) -> bool {
        ffi::value_is_map(&self.inner)
    }

    pub fn is_function(&self) -> bool {
        ffi::value_is_function(&self.inner)
    }

    // ---- Primitive accessors ----
    // Each returns None if the Value isn't the expected type.

    pub fn as_int(&self) -> Option<i64> {
        self.is_int().then(|| ffi::value_get_int(&self.inner))
    }

    pub fn as_float(&self) -> Option<f64> {
        self.is_float().then(|| ffi::value_get_float(&self.inner))
    }

    pub fn as_bool(&self) -> Option<bool> {
        self.is_bool().then(|| ffi::value_get_bool(&self.inner))
    }

    /// Get the raw CxxString reference. Frost strings are byte sequences,
    /// not necessarily UTF-8. Use `.to_str()` for text, `.as_bytes()` for
    /// binary.
    pub fn as_cxx_string(&self) -> Option<&cxx::CxxString> {
        self.is_string().then(|| ffi::value_get_string(&self.inner))
    }

    /// Get the string as UTF-8. Returns None if not a String or if the
    /// bytes aren't valid UTF-8 (Frost strings can hold binary data).
    pub fn as_string(&self) -> Option<&str> {
        self.as_cxx_string().and_then(|s| s.to_str().ok())
    }

    /// Get the string's raw bytes. Works for all Frost strings regardless
    /// of encoding.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        self.as_cxx_string().map(|s| s.as_bytes())
    }

    // ---- Array accessors ----

    /// Number of elements, or None if not an Array.
    pub fn array_len(&self) -> Option<usize> {
        self.is_array().then(|| ffi::value_array_len(&self.inner))
    }

    /// Get element by index. Returns None if not an Array or index is
    /// out of bounds.
    pub fn array_get(&self, index: usize) -> Option<FrostValue> {
        if self.is_array() {
            let val = ffi::value_array_get(&self.inner, index);
            if ffi::value_is_null(&val) { None } else { Some(Self::from_shared(val)) }
        } else {
            None
        }
    }

    /// Zero-copy view of the array's contiguous storage.
    /// The returned slice borrows from this FrostValue -- elements are
    /// `SharedPtr<Value>` that can be wrapped in `FrostValue::from_shared`
    /// for inspection.
    pub fn array_slice(&self) -> Option<&[cxx::SharedPtr<ffi::Value>]> {
        self.is_array().then(|| ffi::value_array_slice(&self.inner))
    }

    // ---- Map accessors ----

    /// Number of entries, or None if not a Map.
    pub fn map_len(&self) -> Option<usize> {
        self.is_map().then(|| ffi::value_map_len(&self.inner))
    }

    /// Check if a string key exists.
    pub fn map_has(&self, key: &str) -> bool {
        if self.is_map() {
            let_cxx_string!(k = key);
            ffi::value_map_has(&self.inner, &k)
        } else {
            false
        }
    }

    /// Look up by string key. Returns None if not a Map or key not found.
    /// Returns Some(null FrostValue) if the key exists but maps to null.
    pub fn map_get(&self, key: &str) -> Option<FrostValue> {
        if self.is_map() {
            let_cxx_string!(k = key);
            if ffi::value_map_has(&self.inner, &k) {
                Some(Self::from_shared(ffi::value_map_get(&self.inner, &k)))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Look up by any key type (Int, Float, Bool, or String).
    /// Returns None if not a Map or key not found.
    /// Returns Some(null FrostValue) if the key exists but maps to null.
    pub fn map_get_by(&self, key: &FrostValue) -> Option<FrostValue> {
        if self.is_map() {
            if ffi::value_map_contains_key(&self.inner, &key.inner) {
                Some(Self::from_shared(ffi::value_map_get_by(&self.inner, &key.inner)))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Zero-copy view of the map's keys (sorted). Parallel with `map_values`.
    /// Iteration: `keys.iter().zip(values.iter())`.
    pub fn map_keys(&self) -> Option<&[cxx::SharedPtr<ffi::Value>]> {
        self.is_map().then(|| ffi::value_map_keys(&self.inner))
    }

    /// Zero-copy view of the map's values. Parallel with `map_keys`.
    pub fn map_values(&self) -> Option<&[cxx::SharedPtr<ffi::Value>]> {
        self.is_map().then(|| ffi::value_map_values(&self.inner))
    }

    // ---- Frost-semantic operations ----
    // These mirror Frost's operators. They follow Frost's type rules:
    // - add works on numerics (Int/Float), strings (concat), arrays, maps
    // - subtract/multiply/divide/modulus are numeric-only
    // - comparisons work on numerics and strings
    // - truthy: only null and false are falsy

    /// Frost `+` operator. Works on numerics, strings (concat), arrays, maps.
    pub fn add(&self, rhs: &FrostValue) -> Result<FrostValue, cxx::Exception> {
        Ok(Self::from_shared(ffi::value_add(&self.inner, &rhs.inner)?))
    }

    /// Frost `-` operator. Numeric only.
    pub fn subtract(&self, rhs: &FrostValue) -> Result<FrostValue, cxx::Exception> {
        Ok(Self::from_shared(ffi::value_subtract(&self.inner, &rhs.inner)?))
    }

    /// Frost `*` operator. Numeric only.
    pub fn multiply(&self, rhs: &FrostValue) -> Result<FrostValue, cxx::Exception> {
        Ok(Self::from_shared(ffi::value_multiply(&self.inner, &rhs.inner)?))
    }

    /// Frost `/` operator. Numeric only. Throws on division by zero.
    pub fn divide(&self, rhs: &FrostValue) -> Result<FrostValue, cxx::Exception> {
        Ok(Self::from_shared(ffi::value_divide(&self.inner, &rhs.inner)?))
    }

    /// Frost `%` operator. Integer only. Throws on modulus by zero.
    pub fn modulus(&self, rhs: &FrostValue) -> Result<FrostValue, cxx::Exception> {
        Ok(Self::from_shared(ffi::value_modulus(&self.inner, &rhs.inner)?))
    }

    /// Frost `==` operator. Deep equality. No cross-type numeric equality
    /// (3 != 3.0).
    pub fn equal(&self, rhs: &FrostValue) -> Result<FrostValue, cxx::Exception> {
        Ok(Self::from_shared(ffi::value_equal(&self.inner, &rhs.inner)?))
    }

    /// Frost `!=` operator.
    pub fn not_equal(&self, rhs: &FrostValue) -> Result<FrostValue, cxx::Exception> {
        Ok(Self::from_shared(ffi::value_not_equal(&self.inner, &rhs.inner)?))
    }

    /// Frost `<` operator. Numeric and string comparison.
    pub fn less_than(&self, rhs: &FrostValue) -> Result<FrostValue, cxx::Exception> {
        Ok(Self::from_shared(ffi::value_less_than(&self.inner, &rhs.inner)?))
    }

    /// Frost `<=` operator.
    pub fn less_than_or_equal(&self, rhs: &FrostValue) -> Result<FrostValue, cxx::Exception> {
        Ok(Self::from_shared(ffi::value_less_than_or_equal(&self.inner, &rhs.inner)?))
    }

    /// Frost `>` operator.
    pub fn greater_than(&self, rhs: &FrostValue) -> Result<FrostValue, cxx::Exception> {
        Ok(Self::from_shared(ffi::value_greater_than(&self.inner, &rhs.inner)?))
    }

    /// Frost `>=` operator.
    pub fn greater_than_or_equal(&self, rhs: &FrostValue) -> Result<FrostValue, cxx::Exception> {
        Ok(Self::from_shared(ffi::value_greater_than_or_equal(&self.inner, &rhs.inner)?))
    }

    /// Frost truthiness. Only `null` and `false` are falsy.
    /// `0`, `""`, `[]`, `{}` are all truthy.
    pub fn truthy(&self) -> bool {
        ffi::value_truthy(&self.inner)
    }

    // ---- Function accessor ----

    /// If this value is a Function, return a `FrostFunction` handle that
    /// can be called. The FrostFunction type proves at the type level that
    /// the value is callable.
    pub fn as_function(&self) -> Option<FrostFunction> {
        self.is_function().then(|| FrostFunction { inner: self.inner.clone() })
    }

    // ---- Destructuring ----

    /// Destructure into a `FrostRef` enum for exhaustive pattern matching.
    /// This is the idiomatic way to dispatch on value type in Rust:
    ///
    /// ```ignore
    /// match value.unpack() {
    ///     FrostRef::Int(n) => println!("got int {n}"),
    ///     FrostRef::Array(elems) => println!("{} elements", elems.len()),
    ///     FrostRef::Function(f) => { f.call_with(&[])? },
    ///     _ => {},
    /// }
    /// ```
    pub fn unpack(&self) -> FrostRef<'_> {
        if self.is_null() {
            FrostRef::Null
        } else if self.is_int() {
            FrostRef::Int(ffi::value_get_int(&self.inner))
        } else if self.is_float() {
            FrostRef::Float(ffi::value_get_float(&self.inner))
        } else if self.is_bool() {
            FrostRef::Bool(ffi::value_get_bool(&self.inner))
        } else if self.is_string() {
            FrostRef::String(ffi::value_get_string(&self.inner))
        } else if self.is_array() {
            FrostRef::Array(ffi::value_array_slice(&self.inner))
        } else if self.is_map() {
            FrostRef::Map {
                keys: ffi::value_map_keys(&self.inner),
                values: ffi::value_map_values(&self.inner),
            }
        } else {
            FrostRef::Function(FrostFunction { inner: self.inner.clone() })
        }
    }

    // ---- Stringification ----

    /// The Frost type name: "Null", "Int", "Float", "Bool", "String",
    /// "Array", "Map", or "Function".
    pub fn type_name(&self) -> String {
        ffi::value_type_name(&self.inner)
            .to_str()
            .unwrap_or("Unknown")
            .to_owned()
    }

    /// Frost's string representation (same as `to_string()` in Frost).
    pub fn to_display_string(&self) -> String {
        ffi::value_to_string(&self.inner)
            .to_str()
            .unwrap_or("")
            .to_owned()
    }

    // ---- Conversions ----

    /// Consume this wrapper, returning the underlying shared pointer.
    /// Used by generated bridge code to pass values back to C++.
    pub fn into_shared(self) -> cxx::SharedPtr<ffi::Value> {
        self.inner
    }

    /// Wrap an existing shared pointer (e.g. received from C++ via a slice).
    pub fn from_shared(inner: cxx::SharedPtr<ffi::Value>) -> Self {
        Self { inner }
    }

    /// Borrow the underlying Value reference. Used by generated code for
    /// operations like `try_downcast` that take `&Value`.
    pub fn as_raw(&self) -> &ffi::Value {
        &self.inner
    }
}

// ===========================================================================
// FrostRef -- the destructured enum for pattern matching
// ===========================================================================

/// Frost's type system expressed as a Rust enum with associated data.
///
/// Obtained via `FrostValue::unpack()`. Borrows from the FrostValue, so
/// the FrostValue must outlive any references taken from the enum.
///
/// The `String` variant carries a `&CxxString` (raw bytes). Call
/// `.to_str()` for UTF-8 text or `.as_bytes()` for binary access.
///
/// The `Array` and `Map` variants carry zero-copy slices into the
/// underlying C++ containers.
pub enum FrostRef<'a> {
    Null,
    Int(i64),
    Float(f64),
    Bool(bool),
    String(&'a cxx::CxxString),
    Array(&'a [cxx::SharedPtr<ffi::Value>]),
    Map {
        keys: &'a [cxx::SharedPtr<ffi::Value>],
        values: &'a [cxx::SharedPtr<ffi::Value>],
    },
    Function(FrostFunction),
}

// ===========================================================================
// FrostFunction -- type-safe callable handle
// ===========================================================================

/// A Frost Function value. Obtained via `FrostValue::as_function()`.
///
/// Provides type-level proof that the value is callable, so `call`
/// doesn't need a runtime type check.
///
/// # Calling Frost functions from Rust
/// ```ignore
/// let f = value.as_function().ok_or("not a function")?;
/// let result = f.call_with(&[FrostValue::from_int(42)])?;
/// ```
///
/// # Creating Frost-callable functions from Rust
/// ```ignore
/// let adder = FrostFunction::new(move |args| {
///     let n = args[0].as_int().ok_or("expected Int")?;
///     Ok(FrostValue::from_int(n + 1))
/// });
/// ```
///
/// When a Rust-created function is called from Frost and returns `Err`,
/// it becomes a `Frost_Recoverable_Error` that Frost's `try_call` can catch.
pub struct FrostFunction {
    inner: cxx::SharedPtr<ffi::Value>,
}

impl FrostFunction {
    /// Create a Frost-callable function from a Rust closure.
    ///
    /// The closure receives a slice of `FrostValue` args (already validated
    /// by REQUIRE_ARGS on the C++ side if called through a registered
    /// extension function). Returns `Ok(value)` or `Err(message)`.
    ///
    /// The resulting `FrostFunction` can be returned to Frost where it
    /// behaves like any native function (callable, passable, storable in
    /// maps).
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&[FrostValue]) -> Result<FrostValue, String> + Send + Sync + 'static,
    {
        let wrapper = move |args: &[cxx::SharedPtr<ffi::Value>]| -> Result<cxx::SharedPtr<ffi::Value>, String> {
            let frost_args: Vec<FrostValue> = args.iter()
                .map(|a| FrostValue::from_shared(a.clone()))
                .collect();
            let result = f(&frost_args)?;
            Ok(result.into_shared())
        };
        let closure = Box::new(RustClosure { f: Box::new(wrapper) });
        Self { inner: ffi::make_closure(closure) }
    }

    /// Call this function with raw SharedPtr args.
    /// Returns Err if the function throws a Frost recoverable error.
    pub fn call(&self, args: &[cxx::SharedPtr<ffi::Value>]) -> Result<FrostValue, cxx::Exception> {
        Ok(FrostValue::from_shared(ffi::value_call(&self.inner, args)?))
    }

    /// Call this function with FrostValue args (convenience wrapper).
    pub fn call_with(&self, args: &[FrostValue]) -> Result<FrostValue, cxx::Exception> {
        let shared_args: Vec<cxx::SharedPtr<ffi::Value>> =
            args.iter().map(|v| v.inner.clone()).collect();
        self.call(&shared_args)
    }

    /// Convert back to a generic FrostValue (for returning from extension
    /// functions).
    pub fn into_value(self) -> FrostValue {
        FrostValue { inner: self.inner }
    }

    /// Consume, returning the underlying shared pointer.
    pub fn into_shared(self) -> cxx::SharedPtr<ffi::Value> {
        self.inner
    }
}

impl std::fmt::Display for FrostValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

impl std::fmt::Debug for FrostValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FrostValue({}:{})", self.type_name(), self.to_display_string())
    }
}
