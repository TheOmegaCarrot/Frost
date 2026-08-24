//! Versioned (de)serialization of a [`CompiledFunction`] tree.
//!
//! Every serialized function carries a [`FormatVersion`] marker: a zero-sized field that
//! stamps the runtime's crate version on save, and whose `Deserialize` rejects an image
//! built by a different version. It is the first field, in the *only* `Deserialize` path, so
//! a stale or mismatched image fails to load: even a bare `from_bytes::<CompiledFunction>`
//! gets the check, with no bypass. (Deserialization yields an *untrusted* `CompiledFunction`
//! regardless; it still earns trust via `assert_trusted` or a future verifier.)
//!
//! The constant pool serializes through [`const_pool`] as tagged [`ConstValue`]s,
//! so it round-trips through any format, including non-self-describing binary ones.

use std::sync::Arc;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{FrostError, FrostFloat, MapKey, Value};

/// The runtime version stamped onto (and required by) a serialized image.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A zero-sized [`CompiledFunction`](crate::CompiledFunction) field that stamps the runtime
/// version on serialize and rejects a mismatch on deserialize. Carries no runtime data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FormatVersion;

impl Serialize for FormatVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        VERSION.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for FormatVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let found = String::deserialize(deserializer)?;
        if found == VERSION {
            Ok(FormatVersion)
        } else {
            Err(D::Error::custom(format!(
                "bytecode image was built by frost-runtime {found}, but this runtime is {VERSION}"
            )))
        }
    }
}

// ============================================================
// ConstValue: the trivially-serializable subset of a Value
// ============================================================

// A constant-pool value: the trivially-serializable subset of a `Value`, tagged so it
// round-trips through non-self-describing formats (which `Value`'s own self-describing
// serde cannot). Functions and opaque handles are excluded: they never appear in a
// constant pool, and converting one is an error rather than a panic.
#[derive(Serialize, Deserialize)]
enum ConstValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(FrostFloat),
    String(Arc<str>),
    Bytes(Arc<[u8]>),
    Array(Vec<ConstValue>),
    Map(Vec<(MapKey, ConstValue)>),
}

impl TryFrom<&Value> for ConstValue {
    type Error = FrostError;

    fn try_from(value: &Value) -> Result<Self, FrostError> {
        Ok(match value {
            Value::Null => ConstValue::Null,
            Value::Bool(b) => ConstValue::Bool(*b),
            Value::Int(i) => ConstValue::Int(*i),
            Value::Float(f) => ConstValue::Float(*f),
            Value::String(s) => ConstValue::String(s.clone()),
            Value::Bytes(b) => ConstValue::Bytes(b.clone()),
            Value::Array(a) => ConstValue::Array(
                a.into_iter()
                    .map(ConstValue::try_from)
                    .collect::<Result<_, _>>()?,
            ),
            Value::Map(m) => ConstValue::Map(
                m.into_iter()
                    .map(|(k, v)| Ok((k.clone(), ConstValue::try_from(v)?)))
                    .collect::<Result<_, FrostError>>()?,
            ),
            Value::NativeFunction(_) | Value::Closure(_) | Value::Opaque(_) => {
                return Err("a function or opaque value cannot be serialized".into());
            }
        })
    }
}

impl From<ConstValue> for Value {
    fn from(c: ConstValue) -> Value {
        match c {
            ConstValue::Null => Value::Null,
            ConstValue::Bool(b) => Value::from(b),
            ConstValue::Int(i) => Value::from(i),
            ConstValue::Float(f) => Value::from(f),
            ConstValue::String(s) => Value::from(s),
            ConstValue::Bytes(b) => Value::from(b),
            ConstValue::Array(items) => Value::from(
                items
                    .into_iter()
                    .map(Value::from)
                    .collect::<crate::FrostArray>(),
            ),
            ConstValue::Map(pairs) => Value::from(
                pairs
                    .into_iter()
                    .map(|(k, v)| (k, Value::from(v)))
                    .collect::<crate::FrostMap>(),
            ),
        }
    }
}

// ============================================================
// const_pool: the `#[serde(with)]` adapter for `constants: Vec<Value>`
// ============================================================

/// Serialize/deserialize a constant pool through [`ConstValue`]. Referenced by
/// `#[serde(with = "serialize::const_pool")]` on `CompiledFunction::constants`; serializing a
/// function-valued constant is an error.
pub(crate) mod const_pool {
    use serde::ser::{Error as _, SerializeSeq};

    use super::{ConstValue, Deserialize, Deserializer, Serializer, Value};

    pub fn serialize<S: Serializer>(constants: &[Value], serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(constants.len()))?;
        for value in constants {
            seq.serialize_element(&ConstValue::try_from(value).map_err(S::Error::custom)?)?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Value>, D::Error> {
        Ok(Vec::<ConstValue>::deserialize(deserializer)?
            .into_iter()
            .map(Value::from)
            .collect())
    }
}
