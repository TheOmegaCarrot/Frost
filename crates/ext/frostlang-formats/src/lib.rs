//! Data formats for Frost: encoding values as text or binary data, and
//! decoding them back, for hosts and scripts alike.
//!
//! Each format is a module of this crate, such as [`json`], compiled in by the
//! Cargo feature of the same name. None is on by default, so a build carries
//! only the parsers of the formats it picks.
//!
//! # Encoding and decoding
//!
//! Every format has the same two functions, in its module here and, for
//! scripts, in its module of the extension:
//!
//! - `encode` gives a value in the format: a String for a text format, Bytes
//!   for a binary one. It takes the format's own options, which a script gives
//!   as a Map, or leaves out.
//! - `decode` takes what `encode` gives, and gives the value it holds.
//!
//! For every value `v` that encodes, `decode(encode(v)) == v`. Decoding and
//! then encoding keeps the data, though not its formatting. The exception is
//! input holding a value Frost can only approximate, as each format's module
//! documents.
//!
//! Encoding a value the format has no form for, such as Bytes in JSON, is an
//! [`EncodeError`] naming it and where it is within the value encoded; so is
//! nesting deeper than the format allows. Decoding malformed input is a
//! [`DecodeError`]. A script gets each as an error raised by the function.
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

mod error;
// The machinery every format shares: unused when no format is compiled in.
// Each `not(feature = ...)` attribute, `Format`, and `Format::ALL` list every
// format's feature; a new format joins each list.
#[cfg_attr(not(feature = "json"), allow(unused))]
mod mapping;

#[cfg(feature = "json")]
pub mod json;

use std::collections::BTreeSet;

use frostlang::{Extension, ExtensionBuilder};

pub use error::{DecodeError, EncodeError};

/// The `ext.formats` extension, holding a module for each format in `formats`
/// whose functions are those of the format's module of this crate; see the
/// [crate docs](crate).
pub fn extension(formats: Formats) -> Extension {
    formats
        .0
        .into_iter()
        .fold(Extension::builder("formats"), |extension, format| {
            format.add_module(extension)
        })
        .build()
}

/// A data format the extension can provide, as a module of `ext.formats`;
/// see the [crate docs](crate) for compiling it in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Format {
    /// JSON, as the module `json`; see [`json`].
    #[cfg(feature = "json")]
    Json,
}

impl Format {
    /// Every format compiled in.
    const ALL: &[Format] = &[
        #[cfg(feature = "json")]
        Format::Json,
    ];

    /// `extension`, with this format's module added.
    // With no format compiled in, the match has no arms and `extension` goes
    // unused.
    #[cfg_attr(not(feature = "json"), allow(unused_variables))]
    fn add_module(self, extension: ExtensionBuilder) -> ExtensionBuilder {
        match self {
            #[cfg(feature = "json")]
            Format::Json => extension.module("json", json::module),
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
