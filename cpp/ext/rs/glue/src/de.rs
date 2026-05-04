//! Serde Deserializer for FrostValue.
//!
//! Allows deserializing Frost values into Rust types using `#[derive(Deserialize)]`.
//!
//! ```ignore
//! use frost_glue::serde::from_value;
//!
//! #[derive(serde::Deserialize)]
//! struct Options {
//!     headers: bool,
//!     #[serde(default = "default_delim")]
//!     delim: String,
//! }
//!
//! let opts: Options = from_value(frost_map_value)?;
//! ```

use crate::{FrostRef, FrostValue};
use std::fmt;

// ===========================================================================
// Error type (required by serde)
// ===========================================================================

/// Deserialization error. Wraps a human-readable message.
#[derive(Debug)]
pub struct Error(String);

impl Error {
    pub fn message(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

impl std::error::Error for Error {}

// ===========================================================================
// Public API
// ===========================================================================

/// Deserialize a FrostValue into any type that implements `serde::Deserialize`.
pub fn from_value<'de, T: serde::Deserialize<'de>>(value: &FrostValue) -> Result<T, Error> {
    // Clone the value (refcount bump) so the Deserializer owns it.
    let de = Deserializer::new(value.clone());
    T::deserialize(de)
}

// ===========================================================================
// Deserializer
// ===========================================================================

/// Serde Deserializer backed by a FrostValue.
///
/// Owns the FrostValue (refcounted, cheap to clone). This avoids lifetime
/// issues when creating sub-deserializers for Array elements and Map values.
pub struct Deserializer {
    value: FrostValue,
}

impl Deserializer {
    pub fn new(value: FrostValue) -> Self {
        Self { value }
    }
}

impl<'de> serde::Deserializer<'de> for Deserializer {
    type Error = Error;

    // The core dispatch: look at the FrostValue variant, call the
    // matching visitor method. Serde's generated Deserialize impls
    // call deserialize_* methods, which (via forward_to_deserialize_any!)
    // all route here.
    fn deserialize_any<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.value.unpack() {
            FrostRef::Null => visitor.visit_unit(),
            FrostRef::Int(n) => visitor.visit_i64(n),
            FrostRef::Float(f) => visitor.visit_f64(f),
            FrostRef::Bool(b) => visitor.visit_bool(b),
            FrostRef::String(s) => {
                let s = s.to_str().map_err(serde::de::Error::custom)?;
                visitor.visit_str(s)
            }
            FrostRef::Array(_) => self.deserialize_seq(visitor),
            FrostRef::Map { .. } => self.deserialize_map(visitor),
            FrostRef::Function(_) => Err(serde::de::Error::custom("cannot deserialize Function")),
        }
    }

    // Array -> sequence. Creates a SeqAccess that yields each element.
    fn deserialize_seq<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let slice = self
            .value
            .array_slice()
            .ok_or_else(|| serde::de::Error::custom("expected Array"))?;
        visitor.visit_seq(FrostSeqAccess { iter: slice.iter() })
    }

    // Map -> map of key-value pairs. Creates a MapAccess that yields entries.
    // Also handles struct deserialization (serde calls deserialize_struct
    // which we forward here via forward_to_deserialize_any, but we also
    // override deserialize_struct explicitly to route through map).
    fn deserialize_map<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let keys = self
            .value
            .map_keys()
            .ok_or_else(|| serde::de::Error::custom("expected Map"))?;
        let values = self
            .value
            .map_values()
            .ok_or_else(|| serde::de::Error::custom("expected Map"))?;
        visitor.visit_map(FrostMapAccess {
            keys: keys.iter(),
            values: values.iter(),
        })
    }

    // Structs are deserialized from Maps (keys = field names).
    fn deserialize_struct<V: serde::de::Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_map(visitor)
    }

    // Option<T>: Frost null -> None, anything else -> Some(T).
    fn deserialize_option<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if self.value.is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    // Forward everything else to deserialize_any.
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64
        char str string bytes byte_buf
        unit unit_struct newtype_struct
        tuple tuple_struct
        enum identifier ignored_any
    }
}

// ===========================================================================
// SeqAccess — iterates Array elements for serde
// ===========================================================================

/// Walks a FrostArray's contiguous SharedPtr slice, yielding each element
/// as a sub-Deserializer.
struct FrostSeqAccess<'a> {
    iter: std::slice::Iter<'a, cxx::SharedPtr<crate::ffi::Value>>,
}

impl<'de, 'a> serde::de::SeqAccess<'de> for FrostSeqAccess<'a> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(ptr) => {
                let de = Deserializer::new(FrostValue::from_shared(ptr.clone()));
                seed.deserialize(de).map(Some)
            }
            None => Ok(None),
        }
    }
}

// ===========================================================================
// MapAccess — iterates Map key-value pairs for serde
// ===========================================================================

/// Walks a FrostMap's parallel key/value slices, yielding each entry
/// as a pair of sub-Deserializers.
struct FrostMapAccess<'a> {
    keys: std::slice::Iter<'a, cxx::SharedPtr<crate::ffi::Value>>,
    values: std::slice::Iter<'a, cxx::SharedPtr<crate::ffi::Value>>,
}

impl<'de, 'a> serde::de::MapAccess<'de> for FrostMapAccess<'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        match self.keys.next() {
            Some(ptr) => {
                let de = Deserializer::new(FrostValue::from_shared(ptr.clone()));
                seed.deserialize(de).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        // Keys and values are always paired (parallel slices from flat_map).
        let ptr = self.values.next().expect("map value missing for key");
        let de = Deserializer::new(FrostValue::from_shared(ptr.clone()));
        seed.deserialize(de)
    }
}
