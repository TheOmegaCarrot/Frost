//! Program images: a compiled program saved as bytes, so it can run later
//! without being compiled again.
//!
//! An image loads only into the same version of this runtime that wrote it.
//!
//! ```
//! # use std::sync::Arc;
//! # use frostlang::image;
//! # fn demo(program: frostlang::TrustedProgram) -> Result<(), image::ImageError> {
//! let bytes = image::encode(&program);
//!
//! // Later, perhaps in another process:
//! let function = image::decode(&bytes)?;
//! // Trust it only if you trust where the bytes came from.
//! let program = Arc::new(function).assert_trusted();
//! # Ok(())
//! # }
//! ```

use std::fmt;

use crate::TrustedProgram;
use crate::bytecode::{CompiledFunction, FormatVersion};
use crate::vm::serialize::VERSION;

/// Begins every image. Frost source never contains a NUL byte, so no script is
/// mistaken for an image.
const MAGIC: &[u8] = b"\0frost image\n";

/// Whether `bytes` are an image rather than Frost source: whether they begin as
/// every image does. Bytes that pass may still fail to [`decode`].
pub fn is_image(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

/// The image of `program`.
///
/// # Panics
///
/// If a constant of `program` is a function or Opaque value, or holds one.
/// The Frost compiler never emits such a constant.
pub fn encode(program: &TrustedProgram) -> Vec<u8> {
    postcard::to_extend(&*program.0, MAGIC.to_vec())
        .expect("an image's constants hold no function or Opaque value")
}

/// The program in `image`.
///
/// The result is untrusted: running it requires
/// [`assert_trusted`](CompiledFunction::assert_trusted), whose obligation is
/// yours, so decode only images whose source you trust.
pub fn decode(image: &[u8]) -> Result<CompiledFunction, ImageError> {
    let body = image.strip_prefix(MAGIC).ok_or(ImageError::NotAnImage)?;
    // Every image begins with the version that wrote it. Read it first, so a
    // mismatch reports the version found rather than failing as damage.
    if postcard::take_from_bytes::<FormatVersion>(body).is_err() {
        return Err(match postcard::take_from_bytes::<String>(body) {
            Ok((found, _)) => ImageError::VersionMismatch { found },
            Err(_) => ImageError::Damaged,
        });
    }
    postcard::from_bytes(body).map_err(|_| ImageError::Damaged)
}

/// Why [`decode`] cannot load an image.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImageError {
    /// The bytes are not an image at all; see [`is_image`].
    NotAnImage,
    /// The image was written by another version of this runtime.
    VersionMismatch {
        /// The version that wrote it.
        found: String,
    },
    /// The image is truncated or corrupt.
    Damaged,
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAnImage => f.write_str("not a Frost image"),
            Self::VersionMismatch { found } => write!(
                f,
                "the image was written by frostlang {found}, but this is {VERSION}"
            ),
            Self::Damaged => f.write_str("the image is damaged"),
        }
    }
}

impl std::error::Error for ImageError {}
