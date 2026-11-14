//! Headers, query parameters, and form fields: checking and encoding what a
//! script gives, and presenting what a server sends.

use std::collections::HashMap;

use frostlang::native::De;
use frostlang::{FrostError, MapKey, Value, frost_fn};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use url::Url;
use url::form_urlencoded::{Serializer, Target};

use crate::config::{HeaderFields, QueryFields};
use crate::error;

/// Headers a script gave, checked: each name lowercase, once, with its values
/// in order. A name may have no values.
#[derive(Clone, Default)]
pub(crate) struct Headers(Vec<(HeaderName, Vec<HeaderValue>)>);

impl Headers {
    /// `fields` checked, for the function `function`.
    pub(crate) fn new(function: &str, fields: HeaderFields) -> Result<Self, FrostError> {
        // Each name as the script spelled it, to name both in an error.
        let mut spellings = HashMap::<HeaderName, String>::new();
        let mut headers = Vec::new();
        for (name, values) in fields {
            let header = HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| error::invalid_header_name(function, &name))?;
            if let Some(first) = spellings.get(&header) {
                return Err(error::headers_differing_in_case(function, first, &name));
            }
            let values = values
                .0
                .iter()
                .map(|value| {
                    HeaderValue::from_bytes(value.as_bytes())
                        .map_err(|_| error::invalid_header_value(function, &name, value))
                })
                .collect::<Result<Vec<_>, _>>()?;
            spellings.insert(header.clone(), name);
            headers.push((header, values));
        }
        Ok(Self(headers))
    }

    /// Whether these headers name `name`.
    fn contains(&self, name: &HeaderName) -> bool {
        self.0.iter().any(|(named, _)| named == name)
    }

    /// The `user-agent` header every client sends unless told otherwise:
    /// `frost-http/` and this crate's version.
    pub(crate) fn user_agent() -> Self {
        let agent = concat!("frost-http/", env!("CARGO_PKG_VERSION"));
        Self(vec![(
            reqwest::header::USER_AGENT,
            vec![HeaderValue::from_static(agent)],
        )])
    }

    /// These headers, joined by each of `defaults` that names a header these
    /// lack.
    pub(crate) fn with_defaults(&self, defaults: &Self) -> Self {
        let defaults = defaults.0.iter().filter(|(name, _)| !self.contains(name));
        Self(self.0.iter().chain(defaults).cloned().collect())
    }

    /// These headers, to send.
    pub(crate) fn to_map(&self) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, values) in &self.0 {
            for value in values {
                map.append(name.clone(), value.clone());
            }
        }
        map
    }
}

/// The headers of a response as a Frost Map: each name lowercase, with its
/// value, or an Array of its values if it is repeated.
pub(crate) fn headers_value(headers: &HeaderMap) -> Value {
    let entries = headers.keys().map(|name| {
        let mut values: Vec<Value> = headers
            .get_all(name)
            .iter()
            .map(|value| text_or_bytes(value.as_bytes().to_vec()))
            .collect();
        let value = match values.len() {
            1 => values.pop().expect("one value"),
            _ => Value::from(values),
        };
        (MapKey::from(name.as_str()), value)
    });
    Value::Map(entries.collect())
}

/// `bytes` as a String if they are UTF-8, else as Bytes, without copying
/// them: a body may be large.
pub(crate) fn text_or_bytes(bytes: Vec<u8>) -> Value {
    String::from_utf8(bytes).map_or_else(|err| Value::from(err.into_bytes()), Value::from)
}

/// Appends `fields` to `url`'s query.
pub(crate) fn append_query(url: &mut Url, fields: &QueryFields) {
    // Only with something to append: opening the query adds a `?` to a URL
    // without one, even if nothing follows.
    if fields.values().any(|values| !values.0.is_empty()) {
        encode(&mut url.query_pairs_mut(), fields);
    }
}

/// Encodes `fields` with `serializer`, as `application/x-www-form-urlencoded`.
fn encode<T: Target>(serializer: &mut Serializer<'_, T>, fields: &QueryFields) {
    for (name, values) in fields {
        for value in &values.0 {
            match value {
                Some(value) => serializer.append_pair(name, value),
                None => serializer.append_key_only(name),
            };
        }
    }
}

/// `form(fields)`: a form body, `fields` URL-encoded.
#[frost_fn]
pub(crate) fn form(fields: De<QueryFields>) -> String {
    let De(fields) = fields;
    let mut serializer = Serializer::new(String::new());
    encode(&mut serializer, &fields);
    serializer.finish()
}
