//! Encoding a Frost value through a format's serializer.

use std::cell::Cell;

use frostlang::{FrostType, MapKey, Value};
use serde::ser::{Error, Serialize, SerializeMap, SerializeSeq, Serializer};

use crate::EncodeError;
use crate::mapping::path::Path;
use crate::mapping::{Mapping, key_source};

/// Why [`encode`] failed.
pub(crate) enum EncodeFailure<E> {
    /// The encoder refused the value.
    Refused(EncodeError),
    /// The serializer failed of its own accord.
    Serializer(E),
}

/// Encodes `value` by `mapping` through `serializer`, the format's own.
pub(crate) fn encode<S: Serializer>(
    mapping: &Mapping,
    value: &Value,
    serializer: S,
) -> Result<S::Ok, EncodeFailure<S::Error>> {
    let refusal = Cell::new(None);
    let encoder = Encoder {
        mapping,
        refusal: &refusal,
        value,
        path: Path::Root,
        depth: 0,
    };
    encoder
        .serialize(serializer)
        .map_err(|err| match refusal.take() {
            Some(refusal) => EncodeFailure::Refused(refusal),
            None => EncodeFailure::Serializer(err),
        })
}

/// A value to encode, and where it sits in the value being encoded. It
/// serializes through a format's serializer by a [`Mapping`], failing on
/// anything the format has no form for.
struct Encoder<'a> {
    mapping: &'a Mapping,
    /// Where a refusal is kept: serde's error type carries only a message
    /// back to [`encode`].
    refusal: &'a Cell<Option<EncodeError>>,
    value: &'a Value,
    path: Path<'a>,
    /// How many Arrays and Maps enclose `value`.
    depth: usize,
}

impl Encoder<'_> {
    /// The encoder for `value`, an element or entry value of this one, at
    /// `path`.
    fn child<'b>(&'b self, value: &'b Value, path: Path<'b>) -> Encoder<'b> {
        Encoder {
            mapping: self.mapping,
            refusal: self.refusal,
            value,
            path,
            depth: self.depth + 1,
        }
    }

    /// Keeps `refusal` for [`encode`], and gives the serializer's error that
    /// stops serializing.
    fn refuse<E: Error>(&self, refusal: EncodeError) -> E {
        let error = E::custom(&refusal);
        self.refusal.set(Some(refusal));
        error
    }

    /// Refuses `what`, which the format has no form for, at this value.
    fn refuse_unrepresentable<E: Error>(&self, what: String) -> E {
        self.refuse(EncodeError::Unrepresentable {
            format: self.mapping.name,
            what,
            path: self.path.to_string(),
        })
    }
}

impl Serialize for Encoder<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mapping = self.mapping;
        let value = self.value;
        match value {
            // Refusals first: they overlap the arms below, so order matters.
            Value::NativeFunction(_) | Value::Closure(_) => {
                Err(self.refuse_unrepresentable(value.type_name().to_string()))
            }
            // An Opaque's rendering names its type and approximates its
            // content, as `<SpecialFloat: NaN>`.
            Value::Opaque(_) => Err(self.refuse_unrepresentable(value.to_debug_string())),
            _ if !value.fits(mapping.types) => {
                Err(self.refuse_unrepresentable(value.type_name().to_string()))
            }
            // Unlike the other refusals, this omits the path: at the limit it
            // runs to over a hundred segments and would bury the reason.
            Value::Array(_) | Value::Map(_) if self.depth == mapping.max_depth => {
                Err(self.refuse(EncodeError::TooDeep {
                    format: mapping.name,
                    limit: mapping.max_depth,
                }))
            }
            // Each type the format has a form for.
            Value::Null => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Int(i) => serializer.serialize_i64(*i),
            Value::Float(f) => serializer.serialize_f64(f.get()),
            Value::String(s) => serializer.serialize_str(s),
            Value::Bytes(b) => serializer.serialize_bytes(b),
            Value::Array(array) => {
                let mut seq = serializer.serialize_seq(Some(array.len()))?;
                for (index, element) in array.iter().enumerate() {
                    seq.serialize_element(&self.child(element, Path::Index(&self.path, index)))?;
                }
                seq.end()
            }
            Value::Map(map) => {
                let mut entries = serializer.serialize_map(Some(map.len()))?;
                for (key, entry) in map {
                    if !mapping.key_types.contains(key_type(key)) {
                        return Err(self.refuse_unrepresentable(format!(
                            "the {} key {}",
                            key_type(key).name(),
                            key_source(key)
                        )));
                    }
                    entries.serialize_entry(
                        &Key(key),
                        &self.child(entry, Path::Key(&self.path, key)),
                    )?;
                }
                entries.end()
            }
        }
    }
}

/// The Frost type of `key`.
fn key_type(key: &MapKey) -> FrostType {
    match key {
        MapKey::Bool(_) => FrostType::Bool,
        MapKey::Int(_) => FrostType::Int,
        MapKey::Float(_) => FrostType::Float,
        MapKey::String(_) => FrostType::String,
        MapKey::Bytes(_) => FrostType::Bytes,
    }
}

/// A Map key, serialized as the value it is.
struct Key<'a>(&'a MapKey);

impl Serialize for Key<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            MapKey::Bool(b) => serializer.serialize_bool(*b),
            MapKey::Int(i) => serializer.serialize_i64(*i),
            MapKey::Float(f) => serializer.serialize_f64(f.get()),
            MapKey::String(s) => serializer.serialize_str(s),
            MapKey::Bytes(b) => serializer.serialize_bytes(b),
        }
    }
}
