use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

/// Frost's String type: immutable text, valid UTF-8 by construction.
///
/// It dereferences to [`str`], so every `&str` method applies, and cloning one
/// is cheap.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct FrostString(Arc<str>);

impl FrostString {
    /// The text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Deref for FrostString {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for FrostString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for FrostString {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for FrostString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&*self.0, f)
    }
}

impl fmt::Display for FrostString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<str> for FrostString {
    fn eq(&self, other: &str) -> bool {
        *self.0 == *other
    }
}

impl PartialEq<&str> for FrostString {
    fn eq(&self, other: &&str) -> bool {
        *self.0 == **other
    }
}

impl From<&str> for FrostString {
    fn from(text: &str) -> Self {
        Self(Arc::from(text))
    }
}

impl From<String> for FrostString {
    fn from(text: String) -> Self {
        Self(Arc::from(text))
    }
}

impl From<Box<str>> for FrostString {
    fn from(text: Box<str>) -> Self {
        Self(Arc::from(text))
    }
}

impl From<Arc<str>> for FrostString {
    fn from(text: Arc<str>) -> Self {
        Self(text)
    }
}

impl From<FrostString> for Arc<str> {
    fn from(text: FrostString) -> Self {
        text.0
    }
}

impl From<FrostString> for String {
    fn from(text: FrostString) -> Self {
        text.0.to_string()
    }
}
