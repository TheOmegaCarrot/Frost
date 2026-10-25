//! A guide to how values convert to and from Rust types.
//!
//! Values cross between Frost and Rust in four ways: a native takes its
//! arguments ([`FromArg`]) and returns its result ([`IntoNativeResult`]), and
//! any serde type converts with [`from_value`] and [`to_value`]. A Rust type
//! converts the same way through every one of these it supports. A native can
//! also take or return any serde type, through [`De`] and [`Ser`].
//!
//! # Types
//!
//! | Rust type | Value | Converts |
//! |---|---|---|
//! | [`Value`] | Any value, as it is | Every way |
//! | `()` | Null | Every way but as an argument |
//! | `bool` | A Bool | Every way |
//! | Each integer type | An Int within the type's range | Every way |
//! | `f64`, [`FrostFloat`] | A Float | Every way |
//! | `f32` | A Float within the `f32` range | Through serde |
//! | `char` | A String of one code point | Every way |
//! | `String` | A String | Every way |
//! | `&str` | A String | As a result, and with [`to_value`] |
//! | `Vec<T>` | An Array of `T` | Every way |
//! | [`Nullable<T>`] | Null, or a `T` | Every way |
//! | [`Opaque<T>`] | An Opaque whose payload is a `T` | Every way |
//! | [`Numeric`], [`Flat`], [`Structured`], [`Primitive`], [`Nonnull`], [`Function`] | A value of that category's types | Every way |
//! | [`FrostString`], [`FrostBytes`], [`FrostArray`], [`FrostMap`] | A String, Bytes, an Array, a Map | As an argument or a result |
//! | [`MapKey`] | A value of a Map key's type | As an argument |
//!
//! A Bytes value is not an Array: a `Vec<u8>` is an Array of Ints. A native
//! takes or returns Bytes as [`FrostBytes`]; a serde type holds them in a field
//! serialized as bytes, such as with the `serde_bytes` crate.
//!
//! Through serde, the shapes of data convert too:
//!
//! - A struct, or a map whose keys are of a Map key's type, is a Map. A
//!   struct's field names are its String keys.
//! - A tuple, a fixed-size array, or a tuple struct is an Array of its
//!   length, or a shorter one where serde fills the fields it lacks with
//!   their `default`.
//! - A newtype struct is the value it wraps, and a unit struct is Null.
//! - An enum's unit variant is a String of its name. Any other variant is a
//!   Map of one entry, from its name to its content. A unit variant is also
//!   read from such an entry holding Null.
//!
//! # Numbers
//!
//! An integer converts only within both its own range and the Int range.
//! Returning one outside the Int range, or passing one to [`to_value`], is an
//! error; so is taking an Int outside the integer type's range, as an argument
//! or through [`from_value`].
//!
//! A float is always finite, as a Float is: a NaN or infinite float crosses
//! only as a [`SpecialFloat`], held by a [`Value`] or an
//! [`Opaque<SpecialFloat>`]. An Int is not a Float; take [`Numeric`] to accept
//! either. The exception to both is serde's
//! [buffered forms](#serdes-buffered-forms).
//!
//! # Absence and Null
//!
//! Frost tells a Map entry holding Null apart from no entry at all, so an
//! `Option` is presence, never Null:
//!
//! - An omitted argument is `None` in an [`Optional`].
//! - A struct field or Map entry left out is `None` to [`from_value`], and one
//!   holding `None` is left out by [`to_value`]. A struct variant's fields
//!   count as a struct's.
//! - To [`from_value`], anything present is `Some`, Null included. Null then
//!   goes to the type the `Option` holds, which must accept Null, as a
//!   [`Value`], a [`Nullable`], a [`Primitive`], or `()` does.
//! - [`to_value`] fails on `None` anywhere but an entry, such as in an Array:
//!   there it has no value. A native's result is never absent, so one that
//!   may be Null is a [`Nullable`] rather than an `Option`.
//!
//! `Option<Nullable<T>>` tells all three apart: absent, Null, and a value. To
//! treat Null as absent, take that type and flatten it with
//! `.and_then(Option::from)`.
//!
//! # Errors
//!
//! An argument the native cannot take is an error worded alike for every
//! native: see [`ArgSite`]. A [`from_value`] or [`to_value`] error names
//! where in the value it happened, written as Frost would index the value, such
//! as `items[2].name`. A [`from_value`] error also names what was found in
//! Frost's terms. Where [`from_value`] judges the value, such as its type, a
//! numeric range, or an Array longer than the target reads, it states what it
//! expected in Frost's terms too. Where the target type judges it, the target
//! describes what it expected in its own words, such as "a nonzero u8" or "a
//! tuple of size 2". To word that for script authors, give your own types a
//! description with serde's `#[serde(expecting = "...")]`.
//!
//! # serde's buffered forms
//!
//! For an untagged, internally tagged, or adjacently tagged enum, or a
//! `#[serde(flatten)]` field, serde first reads the value into an intermediate
//! form of its own and gives the target that instead. The target then reads
//! it by serde's rules, not by [`from_value`]'s:
//!
//! - Null in an `Option` is `None`.
//! - A float takes an Int, or a [`SpecialFloat`] as the float it holds, and
//!   an `f32` rounds a Float beyond its range to an infinity.
//! - A String target takes Bytes, and a bytes target a String.
//! - An integer's range, a `char`'s one code point, and an Array's length are
//!   the target's to judge, in its own words.
//! - A Function, or an Opaque other than a [`SpecialFloat`], anywhere in the
//!   value fails the read, even into a [`Value`] or an [`Opaque<T>`], and even
//!   in a field the target ignores.
//!
//! [`FromArg`]: crate::native::FromArg
//! [`IntoNativeResult`]: crate::native::IntoNativeResult
//! [`De`]: crate::native::De
//! [`Ser`]: crate::native::Ser
//! [`from_value`]: crate::from_value
//! [`to_value`]: crate::to_value
//! [`Value`]: crate::Value
//! [`FrostFloat`]: crate::FrostFloat
//! [`Nullable<T>`]: crate::Nullable
//! [`Nullable`]: crate::Nullable
//! [`Opaque<T>`]: crate::native::Opaque
//! [`Opaque<SpecialFloat>`]: crate::native::Opaque
//! [`Numeric`]: crate::native::Numeric
//! [`Flat`]: crate::native::Flat
//! [`Structured`]: crate::native::Structured
//! [`Primitive`]: crate::native::Primitive
//! [`Nonnull`]: crate::native::Nonnull
//! [`Function`]: crate::native::Function
//! [`FrostString`]: crate::FrostString
//! [`FrostBytes`]: crate::FrostBytes
//! [`FrostArray`]: crate::FrostArray
//! [`FrostMap`]: crate::FrostMap
//! [`MapKey`]: crate::MapKey
//! [`SpecialFloat`]: crate::SpecialFloat
//! [`Optional`]: crate::native::Optional
//! [`ArgSite`]: crate::native::ArgSite
