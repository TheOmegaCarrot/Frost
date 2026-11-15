//! The configuration Maps scripts pass, as they deserialize from Frost values.
//!
//! The types here only check shapes and types; what a value means, such as
//! whether a header name is valid, is checked where it is used.

use std::collections::BTreeMap;
use std::fmt;

use frostlang::native::Flat;
use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};

/// The config `http.client` takes.
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ClientConfig {
    pub(crate) base_url: Option<String>,
    pub(crate) headers: Option<HeaderFields>,
    /// In microseconds.
    pub(crate) timeout: Option<u64>,
    /// In microseconds.
    pub(crate) connect_timeout: Option<u64>,
    /// In microseconds.
    pub(crate) read_timeout: Option<u64>,
    pub(crate) redirects: Option<u64>,
    pub(crate) decompress: Option<bool>,
    pub(crate) verify_tls: Option<bool>,
    /// Each as PEM content.
    pub(crate) ca: Option<Vec<Flat>>,
    pub(crate) system_ca: Option<bool>,
    pub(crate) proxy: Option<ProxySetting>,
}

/// The proxy a client's `proxy` names, or none.
pub(crate) enum ProxySetting {
    /// Every request goes through the proxy at this URL.
    Url(String),
    /// Every request goes directly, whatever proxies the environment names.
    Direct,
}

impl<'de> Deserialize<'de> for ProxySetting {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ProxySettingVisitor;

        impl Visitor<'_> for ProxySettingVisitor {
            type Value = ProxySetting;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("String or false")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<ProxySetting, E> {
                Ok(ProxySetting::Url(value.to_owned()))
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> Result<ProxySetting, E> {
                if value {
                    Err(E::invalid_value(de::Unexpected::Bool(value), &self))
                } else {
                    Ok(ProxySetting::Direct)
                }
            }
        }

        deserializer.deserialize_any(ProxySettingVisitor)
    }
}

/// The config `client.request` takes: a [`RequestOptions`] with a URL and
/// method. Its fields are repeated rather than flattened in, since serde
/// cannot both flatten and deny unknown fields.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RequestConfig {
    url: UrlArg,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    query: Option<QueryFields>,
    #[serde(default)]
    headers: Option<HeaderFields>,
    #[serde(default)]
    body: Option<Flat>,
    #[serde(default)]
    body_file: Option<String>,
    #[serde(default)]
    timeout: Option<u64>,
    #[serde(default)]
    save_to: Option<String>,
}

impl RequestConfig {
    /// The config's URL, its method if given, and the rest of its options.
    pub(crate) fn into_parts(self) -> (UrlArg, Option<String>, RequestOptions) {
        let Self {
            url,
            method,
            query,
            headers,
            body,
            body_file,
            timeout,
            save_to,
        } = self;
        let options = RequestOptions {
            query,
            headers,
            body,
            body_file,
            timeout,
            save_to,
        };
        (url, method, options)
    }
}

/// The options a request takes beyond its URL and method: the whole config
/// of `client.get` and its kin, which take those as their arguments and name.
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct RequestOptions {
    pub(crate) query: Option<QueryFields>,
    pub(crate) headers: Option<HeaderFields>,
    pub(crate) body: Option<Flat>,
    /// A path.
    pub(crate) body_file: Option<String>,
    /// In microseconds.
    pub(crate) timeout: Option<u64>,
    /// A path.
    pub(crate) save_to: Option<String>,
}

/// The options `client.request_all` takes.
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct BatchOptions {
    pub(crate) concurrency: Option<u64>,
}

/// Headers as a script writes them: each name with its value or values.
pub(crate) type HeaderFields = BTreeMap<String, HeaderValues>;

/// A header's value, or the values of a header repeated.
pub(crate) struct HeaderValues(pub(crate) Vec<String>);

impl<'de> Deserialize<'de> for HeaderValues {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct HeaderValuesVisitor;

        impl<'de> Visitor<'de> for HeaderValuesVisitor {
            type Value = HeaderValues;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("String or Array of String")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<HeaderValues, E> {
                Ok(HeaderValues(vec![value.to_owned()]))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<HeaderValues, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<String>()? {
                    values.push(value);
                }
                Ok(HeaderValues(values))
            }
        }

        deserializer.deserialize_any(HeaderValuesVisitor)
    }
}

/// Query parameters or form fields as a script writes them: each name with
/// its value or values.
pub(crate) type QueryFields = BTreeMap<String, QueryValues>;

/// A parameter's values as text, each `None` for a parameter with no value,
/// written by a script as one value or an Array of them.
pub(crate) struct QueryValues(pub(crate) Vec<Option<String>>);

impl<'de> Deserialize<'de> for QueryValues {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct QueryValuesVisitor;

        /// One value, as a parameter given once.
        fn once<E>(value: Result<QueryValue, E>) -> Result<QueryValues, E> {
            value.map(|QueryValue(value)| QueryValues(vec![value]))
        }

        impl<'de> Visitor<'de> for QueryValuesVisitor {
            type Value = QueryValues;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("String, Int, Bool, Null, or an Array of them")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<QueryValues, E> {
                once(QueryValueVisitor.visit_str(value))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<QueryValues, E> {
                once(QueryValueVisitor.visit_i64(value))
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> Result<QueryValues, E> {
                once(QueryValueVisitor.visit_bool(value))
            }

            fn visit_unit<E: de::Error>(self) -> Result<QueryValues, E> {
                once(QueryValueVisitor.visit_unit())
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<QueryValues, A::Error> {
                let mut values = Vec::new();
                while let Some(QueryValue(value)) = seq.next_element()? {
                    values.push(value);
                }
                Ok(QueryValues(values))
            }
        }

        deserializer.deserialize_any(QueryValuesVisitor)
    }
}

/// One of a parameter's values: the only one, or an element of its Array.
struct QueryValue(Option<String>);

impl<'de> Deserialize<'de> for QueryValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(QueryValueVisitor)
    }
}

/// Reads a [`QueryValue`], the one home of the rules for a value's types.
struct QueryValueVisitor;

impl Visitor<'_> for QueryValueVisitor {
    type Value = QueryValue;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("String, Int, Bool, or Null")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<QueryValue, E> {
        Ok(QueryValue(Some(value.to_owned())))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<QueryValue, E> {
        Ok(QueryValue(Some(value.to_string())))
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<QueryValue, E> {
        Ok(QueryValue(Some(value.to_string())))
    }

    fn visit_unit<E: de::Error>(self) -> Result<QueryValue, E> {
        Ok(QueryValue(None))
    }
}

/// A URL as a script gives one: its text, or a URL Map.
pub(crate) enum UrlArg {
    Text(String),
    Parts(UrlParts),
}

impl<'de> Deserialize<'de> for UrlArg {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UrlArgVisitor;

        impl<'de> Visitor<'de> for UrlArgVisitor {
            type Value = UrlArg;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("String or URL Map")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<UrlArg, E> {
                Ok(UrlArg::Text(value.to_owned()))
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<UrlArg, A::Error> {
                UrlParts::deserialize(de::value::MapAccessDeserializer::new(map)).map(UrlArg::Parts)
            }
        }

        deserializer.deserialize_any(UrlArgVisitor)
    }
}

/// A URL Map: the parts of an http or https URL. Each part but the host is
/// percent-encoded, as it appears in the URL's text.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UrlParts {
    pub(crate) host: String,
    pub(crate) path: String,
    pub(crate) tls: bool,
    #[serde(default)]
    pub(crate) port: Option<u16>,
    #[serde(default)]
    pub(crate) query: Option<String>,
    #[serde(default)]
    pub(crate) user: Option<String>,
    #[serde(default)]
    pub(crate) password: Option<String>,
    #[serde(default)]
    pub(crate) fragment: Option<String>,
}
