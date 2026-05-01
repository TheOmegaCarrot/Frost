#[cxx::bridge(namespace = "frst::rs")]
mod ffi {
    unsafe extern "C++" {
        include!("frost-glue.hpp");

        type ValuePtr;

        // ---- Factories ----
        fn value_null() -> SharedPtr<ValuePtr>;
        fn value_from_int(val: i64) -> SharedPtr<ValuePtr>;
        fn value_from_float(val: f64) -> Result<SharedPtr<ValuePtr>>;
        fn value_from_bool(val: bool) -> SharedPtr<ValuePtr>;
        fn value_from_string(val: &CxxString) -> SharedPtr<ValuePtr>;

        // ---- Type checks ----
        fn value_is_null(val: &ValuePtr) -> bool;
        fn value_is_int(val: &ValuePtr) -> bool;
        fn value_is_float(val: &ValuePtr) -> bool;
        fn value_is_bool(val: &ValuePtr) -> bool;
        fn value_is_string(val: &ValuePtr) -> bool;
        fn value_is_array(val: &ValuePtr) -> bool;
        fn value_is_map(val: &ValuePtr) -> bool;
        fn value_is_function(val: &ValuePtr) -> bool;

        // ---- Accessors (unchecked -- use Value wrapper) ----
        fn value_get_int(val: &ValuePtr) -> i64;
        fn value_get_float(val: &ValuePtr) -> f64;
        fn value_get_bool(val: &ValuePtr) -> bool;
        fn value_get_string(val: &ValuePtr) -> &CxxString;

        // ---- Stringification ----
        fn value_to_string(val: &ValuePtr) -> UniquePtr<CxxString>;
        fn value_type_name(val: &ValuePtr) -> UniquePtr<CxxString>;
    }
}

use cxx::let_cxx_string;

/// A Frost value. Wraps a C++ `shared_ptr<const Value>`.
pub struct Value {
    inner: cxx::SharedPtr<ffi::ValuePtr>,
}

impl Value {
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
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", ffi::value_to_string(&self.inner).to_str().unwrap_or(""))
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_name = ffi::value_type_name(&self.inner);
        let display = ffi::value_to_string(&self.inner);
        write!(
            f,
            "Value({}:{})",
            type_name.to_str().unwrap_or("?"),
            display.to_str().unwrap_or("?"),
        )
    }
}
