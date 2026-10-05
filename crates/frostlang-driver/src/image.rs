//! Program images: the files `compile` writes and `run` loads, so a script runs
//! without being compiled again.
//!
//! An image is [`MAGIC`], then the program's [`CompiledFunction`] tree in
//! postcard. The tree begins with the version of the runtime that built it
//! ([`FormatVersion`]), and only that version loads it.

use frostlang_runtime::{CompiledFunction, FormatVersion};

/// Begins every image. Frost source never contains a NUL byte, so no script is
/// mistaken for an image.
const MAGIC: &[u8] = b"\0frost image\n";

/// Whether `bytes` are an image rather than source.
pub(crate) fn is_image(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

/// The image of `function`.
pub(crate) fn encode(function: &CompiledFunction) -> Vec<u8> {
    postcard::to_extend(function, MAGIC.to_vec())
        .expect("compiled code serializes: its constants hold no functions")
}

/// The program in `image`, which [`is_image`] accepted, or why it cannot load.
pub(crate) fn decode(image: &[u8]) -> Result<CompiledFunction, String> {
    let body = image
        .strip_prefix(MAGIC)
        .expect("`is_image` accepted the image");
    // postcard drops the runtime's own message for a version mismatch, so name
    // the image's version here.
    if postcard::take_from_bytes::<FormatVersion>(body).is_err() {
        return Err(match postcard::take_from_bytes::<String>(body) {
            Ok((version, _)) => {
                format!("it was compiled by Frost {version}; compile it again with this Frost")
            }
            Err(_) => "it is damaged".to_string(),
        });
    }
    postcard::from_bytes(body).map_err(|_| "it is damaged".to_string())
}
