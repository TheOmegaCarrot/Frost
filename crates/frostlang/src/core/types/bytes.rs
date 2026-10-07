use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use crate::core::FrostString;

/// Frost's Bytes type: an immutable byte sequence, carrying no encoding
/// guarantee.
///
/// It dereferences to `[u8]`, so every byte-slice method applies, and cloning
/// one is cheap.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct FrostBytes(Arc<[u8]>);

impl FrostBytes {
    /// The bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Deref for FrostBytes {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for FrostBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Borrow<[u8]> for FrostBytes {
    fn borrow(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for FrostBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
    }
}

impl PartialEq<[u8]> for FrostBytes {
    fn eq(&self, other: &[u8]) -> bool {
        *self.0 == *other
    }
}

impl PartialEq<&[u8]> for FrostBytes {
    fn eq(&self, other: &&[u8]) -> bool {
        *self.0 == **other
    }
}

impl From<&[u8]> for FrostBytes {
    fn from(bytes: &[u8]) -> Self {
        Self(Arc::from(bytes))
    }
}

impl<const N: usize> From<[u8; N]> for FrostBytes {
    fn from(bytes: [u8; N]) -> Self {
        Self(Arc::from(bytes))
    }
}

impl From<Vec<u8>> for FrostBytes {
    fn from(bytes: Vec<u8>) -> Self {
        Self(Arc::from(bytes))
    }
}

impl From<Box<[u8]>> for FrostBytes {
    fn from(bytes: Box<[u8]>) -> Self {
        Self(Arc::from(bytes))
    }
}

impl From<Arc<[u8]>> for FrostBytes {
    fn from(bytes: Arc<[u8]>) -> Self {
        Self(bytes)
    }
}

/// The text's UTF-8 bytes, sharing its storage.
impl From<FrostString> for FrostBytes {
    fn from(text: FrostString) -> Self {
        Self(Arc::from(Arc::<str>::from(text)))
    }
}

impl From<FrostBytes> for Arc<[u8]> {
    fn from(bytes: FrostBytes) -> Self {
        bytes.0
    }
}

impl From<FrostBytes> for Vec<u8> {
    fn from(bytes: FrostBytes) -> Self {
        bytes.0.to_vec()
    }
}
