//! Data formats for Frost: encoding values as text or binary data, and
//! decoding them back, for hosts and scripts alike.
//!
//! Each format is a module of this crate, such as `json`, compiled in by the
//! Cargo feature of the same name. None is on by default, so a build carries
//! only the parsers of the formats it picks. The module [`uint64`] holds
//! [`Uint64`](uint64::Uint64), which the formats with unsigned 64-bit
//! integers share; scripts reach it as `ext.formats.u64`.
//!
//! # Encoding and decoding
//!
//! Every format has the same two functions, in its module here and, for
//! scripts, in its module of the extension:
//!
//! - `encode` gives a value in the format: a String for a text format, Bytes
//!   for a binary one. A format that has options takes them too, which a
//!   script gives as a Map, or leaves out.
//! - `decode` takes what `encode` gives, and gives the value it holds.
//!
//! For every value `v` that encodes, `decode(encode(v)) == v`, and decoding
//! and then encoding keeps the data, though not its formatting. Each format's
//! module documents its exceptions, such as a NaN's sign, or input holding a
//! value Frost can only approximate.
//!
//! Encoding is an [`EncodeError`] when the value:
//!
//! - holds something the format has no form for, such as Bytes in JSON; the
//!   error names it and where it is within the value;
//! - is not a document of the format, such as an Int in TOML;
//! - is longer, or nests deeper, than the format allows.
//!
//! Decoding malformed input is a [`DecodeError`]. A script gets each as an
//! error raised by the function.
//!
//! # The extension
//!
//! A host installs the extension with [`extension`], choosing among the
//! formats compiled in, and scripts import each format as a module of
//! `ext.formats`, such as `ext.formats.json`:
//!
//! ```
//! use frostlang::ImporterBuilder;
//! use frostlang_formats::Formats;
//!
//! let importer = ImporterBuilder::new()
//!     .with_extension(frostlang_formats::extension(Formats::default()))
//!     .expect("no other extension is named `formats`")
//!     .build();
//! ```

// Each format uses only part of the machinery the formats share, so only a
// build with every format checks it for dead code. This attribute lists every
// format's feature, as do others such as `Format`; a new format joins each.
#![cfg_attr(
    not(all(
        feature = "cbor",
        feature = "json",
        feature = "msgpack",
        feature = "toml"
    )),
    allow(dead_code, unused_imports, unused_variables)
)]

mod error;
mod mapping;

#[cfg(feature = "cbor")]
pub mod cbor;
#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "msgpack")]
pub mod msgpack;
#[cfg(feature = "toml")]
pub mod toml;
pub mod uint64;

use std::collections::BTreeSet;

use frostlang::{Extension, ExtensionBuilder};

pub use error::{DecodeError, EncodeError};

/// The `ext.formats` extension, holding a module for each format in `formats`
/// whose functions are those of the format's module of this crate; see the
/// [crate docs](crate). With any format that has [`Uint64`](uint64::Uint64)s,
/// it also holds the module `u64`, for handling them.
pub fn extension(formats: Formats) -> Extension {
    let has_uint64s = formats.0.iter().any(|format| format.has_uint64s());
    formats
        .0
        .into_iter()
        .fold(Extension::builder("formats"), |extension, format| {
            format.add_module(extension)
        })
        .with_if(has_uint64s, |extension| {
            extension.module("u64", uint64::module)
        })
        .build()
}

/// A data format the extension can provide, as a module of `ext.formats`;
/// see the [crate docs](crate) for compiling it in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Format {
    /// CBOR, as the module `cbor`; see [`cbor`].
    #[cfg(feature = "cbor")]
    Cbor,
    /// JSON, as the module `json`; see [`json`].
    #[cfg(feature = "json")]
    Json,
    /// MessagePack, as the module `msgpack`; see [`msgpack`].
    #[cfg(feature = "msgpack")]
    Msgpack,
    /// TOML, as the module `toml`; see [`toml`].
    #[cfg(feature = "toml")]
    Toml,
}

impl Format {
    /// Every format compiled in.
    const ALL: &[Format] = &[
        #[cfg(feature = "cbor")]
        Format::Cbor,
        #[cfg(feature = "json")]
        Format::Json,
        #[cfg(feature = "msgpack")]
        Format::Msgpack,
        #[cfg(feature = "toml")]
        Format::Toml,
    ];

    /// How the format's data maps to Frost values.
    fn mapping(self) -> &'static mapping::Mapping {
        match self {
            #[cfg(feature = "cbor")]
            Format::Cbor => &cbor::CBOR,
            #[cfg(feature = "json")]
            Format::Json => &json::JSON,
            #[cfg(feature = "msgpack")]
            Format::Msgpack => &msgpack::MSGPACK,
            #[cfg(feature = "toml")]
            Format::Toml => &toml::TOML,
        }
    }

    /// Whether the format has unsigned 64-bit integers, which may decode to a
    /// [`Uint64`](uint64::Uint64).
    fn has_uint64s(self) -> bool {
        self.mapping()
            .opaques
            .contains(&mapping::FormatOpaque::Uint64)
    }

    /// `extension`, with this format's module added.
    fn add_module(self, extension: ExtensionBuilder) -> ExtensionBuilder {
        match self {
            #[cfg(feature = "cbor")]
            Format::Cbor => extension.module("cbor", cbor::module),
            #[cfg(feature = "json")]
            Format::Json => extension.module("json", json::module),
            #[cfg(feature = "msgpack")]
            Format::Msgpack => extension.module("msgpack", msgpack::module),
            #[cfg(feature = "toml")]
            Format::Toml => extension.module("toml", toml::module),
        }
    }
}

/// The formats for [`extension`] to provide.
///
/// The [`Default`] is every format compiled in, as [`all`](Self::all) is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Formats(BTreeSet<Format>);

impl Formats {
    /// Every format compiled in.
    pub fn all() -> Self {
        Self(Format::ALL.iter().copied().collect())
    }

    /// No formats.
    pub fn none() -> Self {
        Self(BTreeSet::new())
    }

    /// These formats, with `format` among them.
    #[must_use]
    pub fn with(mut self, format: Format) -> Self {
        self.0.insert(format);
        self
    }

    /// These formats, without `format`.
    #[must_use]
    pub fn without(mut self, format: Format) -> Self {
        self.0.remove(&format);
        self
    }

    /// Whether `format` is among these formats.
    pub fn contains(&self, format: Format) -> bool {
        self.0.contains(&format)
    }
}

impl Default for Formats {
    fn default() -> Self {
        Self::all()
    }
}
