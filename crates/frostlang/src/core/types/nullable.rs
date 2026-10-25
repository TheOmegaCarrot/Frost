use std::fmt;
use std::marker::PhantomData;

use serde::de::value::{EnumAccessDeserializer, MapAccessDeserializer, SeqAccessDeserializer};
use serde::de::{self, IntoDeserializer, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::core::serde::NULLABLE_NEWTYPE_TOKEN;

/// A value that may be Null as well as one of `T`'s types: `Nullable(None)`
/// when it is Null.
///
/// It is how Rust takes or gives a value that may be Null, wherever values
/// convert: as a native's argument or result, and through [`from_value`] and
/// [`to_value`]. An `Option`, by contrast, is presence, never Null; see
/// [Absence and Null](crate::_conversions#absence-and-null).
///
/// # Examples
///
/// ```
/// use frostlang::{Nullable, Value, from_value};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Options {
///     width: Option<Nullable<i64>>,
/// }
///
/// let width = |options: Value| from_value::<Options>(options).unwrap().width;
/// assert_eq!(width(Value::map::<&str, 0>([])), None);
/// assert_eq!(
///     width(Value::map([("width", Value::Null)])),
///     Some(Nullable(None))
/// );
/// assert_eq!(
///     width(Value::map([("width", Value::Int(80))])),
///     Some(Nullable(Some(80)))
/// );
/// ```
///
/// [`from_value`]: crate::from_value
/// [`to_value`]: crate::to_value
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Nullable<T>(pub Option<T>);

impl<T> From<Option<T>> for Nullable<T> {
    fn from(option: Option<T>) -> Self {
        Self(option)
    }
}

impl<T> From<Nullable<T>> for Option<T> {
    fn from(Nullable(option): Nullable<T>) -> Self {
        option
    }
}

/// Serializes `Nullable(None)` as Null, and anything else as what it holds;
/// through another format's serializer, as that format's option.
impl<T: Serialize> Serialize for Nullable<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Through the token newtype, so our own serializer can write `None`
        // as Null; a foreign serializer writes a newtype holding an option.
        serializer.serialize_newtype_struct(NULLABLE_NEWTYPE_TOKEN, &self.0)
    }
}

/// Deserializes Null as `Nullable(None)`, and anything else as `T`; through
/// another format's deserializer, as that format's option.
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Nullable<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Through the token newtype, so our own deserializer can answer
        // whether the value is Null, as it does for no `Option`.
        deserializer
            .deserialize_newtype_struct(NULLABLE_NEWTYPE_TOKEN, NullableVisitor(PhantomData))
    }
}

/// Takes a [`Nullable`] as each kind of deserializer gives it:
///
/// - our own, by `visit_none` or `visit_some`;
/// - a format that passes newtypes through, such as JSON, by
///   `visit_newtype_struct`, holding that format's option;
/// - one that hands over a newtype's value directly, as serde's own value
///   deserializers do, by that value's `visit_*`.
struct NullableVisitor<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> NullableVisitor<T> {
    /// Deserializes `T` from `deserializer`, which holds a value that is not
    /// Null.
    fn present<D: Deserializer<'de>>(deserializer: D) -> Result<Nullable<T>, D::Error> {
        T::deserialize(deserializer).map(|value| Nullable(Some(value)))
    }
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for NullableVisitor<T> {
    type Value = Nullable<T>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a value or Null")
    }

    fn visit_none<E>(self) -> Result<Nullable<T>, E> {
        Ok(Nullable(None))
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Nullable<T>, D::Error> {
        Self::present(deserializer)
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Nullable<T>, D::Error> {
        Option::<T>::deserialize(deserializer).map(Nullable)
    }

    // The value of a newtype handed over directly.

    fn visit_unit<E>(self) -> Result<Nullable<T>, E> {
        Ok(Nullable(None))
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Nullable<T>, E> {
        Self::present(v.into_deserializer())
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Nullable<T>, E> {
        Self::present(v.into_deserializer())
    }

    fn visit_i128<E: de::Error>(self, v: i128) -> Result<Nullable<T>, E> {
        Self::present(v.into_deserializer())
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Nullable<T>, E> {
        Self::present(v.into_deserializer())
    }

    fn visit_u128<E: de::Error>(self, v: u128) -> Result<Nullable<T>, E> {
        Self::present(v.into_deserializer())
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Nullable<T>, E> {
        Self::present(v.into_deserializer())
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Nullable<T>, E> {
        Self::present(v.into_deserializer())
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Nullable<T>, E> {
        Self::present(v.into_deserializer())
    }

    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Nullable<T>, E> {
        Self::present(v.into_deserializer())
    }

    fn visit_seq<A: de::SeqAccess<'de>>(self, seq: A) -> Result<Nullable<T>, A::Error> {
        Self::present(SeqAccessDeserializer::new(seq))
    }

    fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<Nullable<T>, A::Error> {
        Self::present(MapAccessDeserializer::new(map))
    }

    fn visit_enum<A: de::EnumAccess<'de>>(self, data: A) -> Result<Nullable<T>, A::Error> {
        Self::present(EnumAccessDeserializer::new(data))
    }
}
