#[cxx::bridge(namespace = "frst::rs")]
pub mod ffi {
    unsafe extern "C++" {
        include!("frost-glue.hpp");

        type Value;

        // ---- Factories ----
        fn value_null() -> SharedPtr<Value>;
        fn value_from_int(val: i64) -> SharedPtr<Value>;
        fn value_from_float(val: f64) -> Result<SharedPtr<Value>>;
        fn value_from_bool(val: bool) -> SharedPtr<Value>;
        fn value_from_string(val: &CxxString) -> SharedPtr<Value>;

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

        // ---- Function call (caller must verify is_function first) ----
        fn value_call(
            callable: &Value,
            args: &[SharedPtr<Value>],
        ) -> Result<SharedPtr<Value>>;

        // ---- Closure creation ----
        fn make_closure(closure: Box<RustClosure>) -> SharedPtr<Value>;

        // ---- Stringification ----
        fn value_to_string(val: &Value) -> UniquePtr<CxxString>;
        fn value_type_name(val: &Value) -> UniquePtr<CxxString>;
    }

    extern "Rust" {
        type RustClosure;
        fn rust_closure_call(
            closure: &RustClosure,
            args: &[SharedPtr<Value>],
        ) -> Result<SharedPtr<Value>>;
    }
}

use cxx::let_cxx_string;

type ClosureFn = Box<dyn Fn(&[cxx::SharedPtr<ffi::Value>]) -> Result<cxx::SharedPtr<ffi::Value>, String>>;

pub struct RustClosure {
    f: ClosureFn,
}

fn rust_closure_call(
    closure: &RustClosure,
    args: &[cxx::SharedPtr<ffi::Value>],
) -> Result<cxx::SharedPtr<ffi::Value>, String> {
    (closure.f)(args)
}

/// A Frost value. Wraps a C++ `shared_ptr<const Value>`.
pub struct FrostValue {
    inner: cxx::SharedPtr<ffi::Value>,
}

impl FrostValue {
    // ---- Factories ----

    pub fn null() -> Self {
        Self { inner: ffi::value_null() }
    }

    pub fn from_int(val: i64) -> Self {
        Self { inner: ffi::value_from_int(val) }
    }

    /// Returns `Err` if the value is NaN or infinity.
    pub fn from_float(val: f64) -> Result<Self, cxx::Exception> {
        Ok(Self { inner: ffi::value_from_float(val)? })
    }

    pub fn from_bool(val: bool) -> Self {
        Self { inner: ffi::value_from_bool(val) }
    }

    pub fn from_string(val: &str) -> Self {
        let_cxx_string!(s = val);
        Self { inner: ffi::value_from_string(&s) }
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

    pub fn as_int(&self) -> Option<i64> {
        self.is_int().then(|| ffi::value_get_int(&self.inner))
    }

    pub fn as_float(&self) -> Option<f64> {
        self.is_float().then(|| ffi::value_get_float(&self.inner))
    }

    pub fn as_bool(&self) -> Option<bool> {
        self.is_bool().then(|| ffi::value_get_bool(&self.inner))
    }

    pub fn as_string(&self) -> Option<&str> {
        if self.is_string() {
            ffi::value_get_string(&self.inner).to_str().ok()
        } else {
            None
        }
    }

    // ---- Array accessors ----

    pub fn array_len(&self) -> Option<usize> {
        self.is_array().then(|| ffi::value_array_len(&self.inner))
    }

    pub fn array_get(&self, index: usize) -> Option<FrostValue> {
        if self.is_array() {
            let val = ffi::value_array_get(&self.inner, index);
            if ffi::value_is_null(&val) { None } else { Some(Self::from_shared(val)) }
        } else {
            None
        }
    }

    /// Zero-copy view of the array's contents as a slice of shared pointers.
    pub fn array_slice(&self) -> Option<&[cxx::SharedPtr<ffi::Value>]> {
        self.is_array().then(|| ffi::value_array_slice(&self.inner))
    }

    // ---- Map accessors ----

    pub fn map_len(&self) -> Option<usize> {
        self.is_map().then(|| ffi::value_map_len(&self.inner))
    }

    pub fn map_has(&self, key: &str) -> bool {
        if self.is_map() {
            let_cxx_string!(k = key);
            ffi::value_map_has(&self.inner, &k)
        } else {
            false
        }
    }

    pub fn map_get(&self, key: &str) -> Option<FrostValue> {
        if self.is_map() {
            let_cxx_string!(k = key);
            let val = ffi::value_map_get(&self.inner, &k);
            if ffi::value_is_null(&val) {
                None
            } else {
                Some(Self::from_shared(val))
            }
        } else {
            None
        }
    }

    // ---- Function accessor ----

    pub fn as_function(&self) -> Option<FrostFunction> {
        self.is_function().then(|| FrostFunction { inner: self.inner.clone() })
    }

    // ---- Stringification ----

    pub fn type_name(&self) -> String {
        ffi::value_type_name(&self.inner)
            .to_str()
            .unwrap_or("Unknown")
            .to_owned()
    }

    pub fn to_display_string(&self) -> String {
        ffi::value_to_string(&self.inner)
            .to_str()
            .unwrap_or("")
            .to_owned()
    }

    // ---- Conversions ----

    /// Consume this wrapper, returning the underlying shared pointer.
    pub fn into_shared(self) -> cxx::SharedPtr<ffi::Value> {
        self.inner
    }

    /// Wrap an existing shared pointer.
    pub fn from_shared(inner: cxx::SharedPtr<ffi::Value>) -> Self {
        Self { inner }
    }
}

/// A Frost Function value. Invariant: the wrapped value is callable.
///
/// Obtained via `FrostValue::as_function()`. Provides type-level proof
/// that the value is a function, so `call` doesn't need a runtime check.
pub struct FrostFunction {
    inner: cxx::SharedPtr<ffi::Value>,
}

impl FrostFunction {
    /// Create a Frost-callable function from a Rust closure.
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&[FrostValue]) -> Result<FrostValue, String> + 'static,
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

    pub fn call(&self, args: &[cxx::SharedPtr<ffi::Value>]) -> Result<FrostValue, cxx::Exception> {
        Ok(FrostValue::from_shared(ffi::value_call(&self.inner, args)?))
    }

    pub fn call_with(&self, args: &[FrostValue]) -> Result<FrostValue, cxx::Exception> {
        let shared_args: Vec<cxx::SharedPtr<ffi::Value>> =
            args.iter().map(|v| v.inner.clone()).collect();
        self.call(&shared_args)
    }

    /// Convert back to a generic FrostValue.
    pub fn into_value(self) -> FrostValue {
        FrostValue { inner: self.inner }
    }

    /// Consume this wrapper, returning the underlying shared pointer.
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
