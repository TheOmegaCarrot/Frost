//! URLs: `parse_url` and `build_url`, and resolving a request's URL against
//! its client's `base_url`.

use frostlang::native::De;
use frostlang::{FrostResult, FrostString, NativeCtx, Value, frost_fn};
use url::Url;

use crate::config::{UrlArg, UrlParts};
use crate::error;

/// Whether `url` is one this extension can request.
fn is_http(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
}

/// `parse_url(url)`: the URL Map of an http or https URL's text.
#[frost_fn]
pub(crate) fn parse_url(url: FrostString) -> Value {
    match Url::parse(&url) {
        Ok(url) if is_http(&url) => url_map(&url),
        _ => Value::Null,
    }
}

/// The URL Map of `url`, an http or https URL.
fn url_map(url: &Url) -> Value {
    let mut map = vec![
        ("host", Value::from(url.host_str().unwrap_or_default())),
        ("path", Value::from(url.path())),
        ("tls", Value::Bool(url.scheme() == "https")),
    ];
    if let Some(port) = url.port() {
        map.push(("port", Value::Int(i64::from(port))));
    }
    if let Some(query) = url.query() {
        map.push(("query", Value::from(query)));
    }
    if !url.username().is_empty() {
        map.push(("user", Value::from(url.username())));
    }
    if let Some(password) = url.password() {
        map.push(("password", Value::from(password)));
    }
    if let Some(fragment) = url.fragment() {
        map.push(("fragment", Value::from(fragment)));
    }
    Value::Map(
        map.into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect(),
    )
}

/// `build_url(parts)`: the text of the URL a URL Map describes.
#[frost_fn]
pub(crate) fn build_url(#[frost(ctx)] ctx: &mut NativeCtx<'_>, parts: De<UrlParts>) -> FrostResult {
    let De(parts) = parts;
    Ok(Value::from(url_from_parts(ctx.name(), &parts)?.as_str()))
}

/// The URL `parts` describes, for the function `function`.
fn url_from_parts(function: &str, parts: &UrlParts) -> Result<Url, frostlang::FrostError> {
    let scheme = if parts.tls { "https" } else { "http" };
    // Built by parts, so that no part's text can spill into another.
    let mut url = Url::parse(&format!("{scheme}://host.invalid"))
        .expect("a fixed URL with a placeholder host parses");
    // The URL standard's host setter keeps only what comes before a `:`
    // outside brackets, taking the rest for a port it then ignores.
    if parts.host.contains(':') && !parts.host.starts_with('[') {
        return Err(error::invalid_host(
            function,
            &parts.host,
            "a port goes in `port`, and an IPv6 address in brackets",
        ));
    }
    url.set_host(Some(&parts.host))
        .map_err(|reason| error::invalid_host(function, &parts.host, reason))?;
    url.set_port(parts.port)
        .expect("an http or https URL with a host takes any port");
    url.set_path(&parts.path);
    url.set_query(parts.query.as_deref());
    url.set_username(parts.user.as_deref().unwrap_or_default())
        .expect("an http or https URL with a host takes a user");
    url.set_password(parts.password.as_deref())
        .expect("an http or https URL with a host takes a password");
    url.set_fragment(parts.fragment.as_deref());
    Ok(url)
}

/// A client's `base_url`, checked and ready for [`request_url`] to join
/// relative URLs onto, for the function `function`.
pub(crate) fn check_base_url(function: &str, base: &str) -> Result<String, frostlang::FrostError> {
    let url = Url::parse(base).map_err(|reason| error::invalid_url(function, base, reason))?;
    if !is_http(&url) {
        return Err(error::not_http(function, base));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(error::base_url_with_extras(function, base));
    }
    let mut base = String::from(url);
    if !base.ends_with('/') {
        base.push('/');
    }
    Ok(base)
}

/// The URL `url` names, joined onto `base` from [`check_base_url`] if it is
/// relative, for the function `function`.
///
/// The join is plain, as for a directory: the base, then the relative URL
/// without any leading `/`.
pub(crate) fn request_url(
    function: &str,
    base: Option<&str>,
    url: &UrlArg,
) -> Result<Url, frostlang::FrostError> {
    let text = match url {
        UrlArg::Parts(parts) => return url_from_parts(function, parts),
        UrlArg::Text(text) => text,
    };
    let url = match (Url::parse(text), base) {
        (Ok(url), _) => url,
        (Err(url::ParseError::RelativeUrlWithoutBase), Some(base)) => {
            let joined = format!("{base}{}", text.trim_start_matches('/'));
            Url::parse(&joined).map_err(|reason| error::invalid_url(function, &joined, reason))?
        }
        (Err(url::ParseError::RelativeUrlWithoutBase), None) => {
            return Err(error::relative_without_base(function, text));
        }
        (Err(reason), _) => return Err(error::invalid_url(function, text, reason)),
    };
    if is_http(&url) {
        Ok(url)
    } else {
        Err(error::not_http(function, text))
    }
}
