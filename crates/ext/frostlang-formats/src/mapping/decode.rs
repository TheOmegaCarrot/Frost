//! Decoding a Frost value through a format's deserializer.

use std::cell::Cell;
use std::fmt;

use frostlang::value_map::Entry;
use frostlang::{FrostArray, FrostType, MapKey, Value, ValueMap};
use serde::de::{DeserializeSeed, Deserializer, Error, MapAccess, SeqAccess, Visitor};

use crate::mapping::{BeyondInt, Mapping, key_source};

/// The most Array elements to reserve room for before reading them. The
/// input's size hint may claim far more elements than it holds, at every
/// level of nesting, so only a little is trusted in advance.
const MAX_PREALLOCATION: usize = 4096;

/// Why [`decode`] failed.
pub(crate) struct DecodeFailure<E> {
    /// The deserializer's error. For a refusal, it wraps the refusal's text as
    /// the deserializer wraps any message, with the input position if the
    /// deserializer adds one.
    pub(crate) error: E,
    /// The decoder's reason, if it refused what the input holds; `None` if
    /// the deserializer failed of its own accord.
    pub(crate) refusal: Option<String>,
}

/// Decodes a value by `mapping` through `deserializer`, the format's own.
pub(crate) fn decode<'de, D: Deserializer<'de>>(
    mapping: &Mapping,
    deserializer: D,
) -> Result<Value, DecodeFailure<D::Error>> {
    let refusal = Cell::new(None);
    let decoder = Decoder {
        mapping,
        refusal: &refusal,
        depth: 0,
        is_key: false,
    };
    decoder
        .deserialize(deserializer)
        .map_err(|error| DecodeFailure {
            error,
            refusal: refusal.take(),
        })
}

/// Decodes a value by a [`Mapping`], failing on anything the mapping or a
/// Frost value cannot hold.
#[derive(Clone, Copy)]
struct Decoder<'a> {
    mapping: &'a Mapping,
    /// Where a refusal is kept: serde's error type carries only a message
    /// back to [`decode`], which a deserializer may word as it likes.
    refusal: &'a Cell<Option<String>>,
    /// How many Arrays and Maps enclose the value to decode. The decoder
    /// keeps its own count, rather than trusting the parser's limit, which a
    /// Cargo feature of the parser may lift.
    depth: usize,
    /// Whether the value to decode is a Map key, which cannot be an Array or
    /// Map: one is refused at once, rather than decoded first, so that keys
    /// cannot nest past the depth limit.
    is_key: bool,
}

impl Decoder<'_> {
    /// The decoder for a key of the Map being visited.
    fn key_decoder(self) -> Self {
        Self {
            is_key: true,
            ..self
        }
    }

    /// Keeps `reason` for [`decode`], and gives the deserializer's error that
    /// stops deserializing.
    fn refuse<E: Error>(self, reason: impl fmt::Display) -> E {
        let reason = reason.to_string();
        let error = E::custom(&reason);
        self.refusal.set(Some(reason));
        error
    }

    /// Refuses a Map key that `what` describes, which no Frost Map key can
    /// be. Worded as the core words its own refusal of such a key.
    fn refuse_invalid_key<E: Error>(self, what: impl fmt::Display) -> E {
        self.refuse(format_args!("{what} is not a valid Map key"))
    }

    /// Refuses the Array or Map being visited if it is a Map key, before
    /// anything within it is read.
    fn refuse_if_key<E: Error>(self, container: FrostType) -> Result<(), E> {
        if self.is_key {
            return Err(self.refuse_invalid_key(format_args!("Type {}", container.name())));
        }
        Ok(())
    }

    /// Enters the Array or Map being visited: the decoder for its elements or
    /// entry values, or an error if it nests deeper than the format allows.
    fn enter_container<E: Error>(self) -> Result<Self, E> {
        if self.depth == self.mapping.max_depth {
            return Err(self.refuse(format_args!(
                "Arrays and Maps nest more than {} deep",
                self.mapping.max_depth
            )));
        }
        Ok(Self {
            depth: self.depth + 1,
            ..self
        })
    }

    /// The value the integer `n` decodes to: an Int, or what the mapping
    /// makes of an integer beyond the Int range. `nearest` is the Float
    /// nearest `n`.
    fn integer<E: Error, N>(self, n: N, nearest: f64) -> Result<Value, E>
    where
        N: fmt::Display + Copy,
        i64: TryFrom<N>,
        u64: TryFrom<N>,
    {
        if let Ok(n) = i64::try_from(n) {
            return Ok(Value::Int(n));
        }
        let beyond_range = || self.refuse(format_args!("integer {n} is beyond the Int range"));
        match self.mapping.beyond_int {
            BeyondInt::NearestFloat => Ok(Value::from_f64(nearest)),
            BeyondInt::Refused => Err(beyond_range()),
            BeyondInt::Uint64 => match u64::try_from(n) {
                Ok(n) => Ok(crate::uint64::to_value(n)),
                Err(_) => Err(beyond_range()),
            },
        }
    }

    /// Adds to `entries` the entry keyed `key`, whose value `map` reads next.
    fn insert_entry<'de, A: MapAccess<'de>>(
        self,
        entries: &mut ValueMap,
        key: Value,
        map: &mut A,
    ) -> Result<(), A::Error> {
        if !key.fits(MapKey::TYPES) {
            // A key may decode to an Opaque, as a NaN does to a SpecialFloat;
            // its rendering says more than its type.
            let what = match &key {
                Value::Opaque(_) => key.to_debug_string(),
                _ => format!("Type {}", key.type_name()),
            };
            return Err(self.refuse_invalid_key(what));
        }
        let key = MapKey::try_from(key).expect("the key is of a Map key type");
        match entries.entry(key) {
            Entry::Occupied(entry) => {
                Err(self.refuse(format_args!("duplicate key {}", key_source(entry.key()))))
            }
            Entry::Vacant(entry) => {
                entry.insert(map.next_value_seed(self)?);
                Ok(())
            }
        }
    }
}

/// The `f64` equal to `f`, or for a NaN, the NaN of the same sign and
/// payload. Rust leaves a cast's NaN unspecified, so the bits are widened by
/// hand.
fn widen(f: f32) -> f64 {
    if !f.is_nan() {
        return f64::from(f);
    }
    let bits = u64::from(f.to_bits());
    let sign = bits >> 31;
    let payload = bits & 0x007f_ffff;
    // Shifted to the top of the wider mantissa, the quiet bit stays the quiet
    // bit.
    f64::from_bits((sign << 63) | (0x7ff << 52) | (payload << 29))
}

impl<'de> DeserializeSeed<'de> for Decoder<'_> {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Decoder<'_> {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a {} value", self.mapping.name)
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        self.deserialize(deserializer)
    }

    // MessagePack's deserializer passes an extension type as a newtype
    // struct; Frost has no form for one.
    fn visit_newtype_struct<D: Deserializer<'de>>(self, _: D) -> Result<Value, D::Error> {
        Err(self.refuse("an extension type has no Frost form"))
    }

    fn visit_bool<E>(self, b: bool) -> Result<Value, E> {
        Ok(Value::Bool(b))
    }

    fn visit_i64<E>(self, n: i64) -> Result<Value, E> {
        Ok(Value::Int(n))
    }

    fn visit_u64<E: Error>(self, n: u64) -> Result<Value, E> {
        self.integer(n, n as f64)
    }

    fn visit_i128<E: Error>(self, n: i128) -> Result<Value, E> {
        self.integer(n, n as f64)
    }

    fn visit_u128<E: Error>(self, n: u128) -> Result<Value, E> {
        self.integer(n, n as f64)
    }

    fn visit_f32<E>(self, f: f32) -> Result<Value, E> {
        Ok(Value::from_f64(widen(f)))
    }

    fn visit_f64<E>(self, f: f64) -> Result<Value, E> {
        Ok(Value::from_f64(f))
    }

    fn visit_str<E>(self, s: &str) -> Result<Value, E> {
        Ok(Value::from(s))
    }

    fn visit_string<E>(self, s: String) -> Result<Value, E> {
        Ok(Value::from(s))
    }

    fn visit_bytes<E>(self, b: &[u8]) -> Result<Value, E> {
        Ok(Value::from(b))
    }

    fn visit_byte_buf<E>(self, b: Vec<u8>) -> Result<Value, E> {
        Ok(Value::from(b))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        self.refuse_if_key(FrostType::Array)?;
        let element_decoder = self.enter_container()?;
        let hint = seq.size_hint().unwrap_or(0).min(MAX_PREALLOCATION);
        let mut elements = Vec::with_capacity(hint);
        while let Some(element) = seq.next_element_seed(element_decoder)? {
            elements.push(element);
        }
        Ok(Value::from(FrostArray::from(elements)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        self.refuse_if_key(FrostType::Map)?;
        // The first key is read apart from the rest, since it may be a marker.
        let key_decoder = self.key_decoder();
        let Some(first) = map.next_key_seed(key_decoder)? else {
            // An empty Map counts toward the depth all the same, as an empty
            // Array does.
            self.enter_container()?;
            return Ok(Value::from(ValueMap::new()));
        };
        // A value serde has no type for arrives as a one-entry Map whose key
        // is a private marker; see `Mapping::marked`. It is no Frost Map, so
        // it nests nothing.
        if let Value::String(key) = &first
            && let Some(decode_text) = self.mapping.marked_decoder(key)
        {
            let text: String = map.next_value()?;
            return decode_text(&text).map_err(|reason| self.refuse(reason));
        }
        let entry_decoder = self.enter_container()?;
        let mut entries = ValueMap::new();
        entry_decoder.insert_entry(&mut entries, first, &mut map)?;
        while let Some(key) = map.next_key_seed(key_decoder)? {
            entry_decoder.insert_entry(&mut entries, key, &mut map)?;
        }
        Ok(Value::from(entries))
    }
}
