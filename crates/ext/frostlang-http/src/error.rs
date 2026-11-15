//! The errors the extension's functions raise, worded in one place.
//!
//! Each names the function raising it as `function`.

use frostlang::{FrostError, Value};

/// `text` as a script would write it, quoted.
pub(crate) fn quoted(text: &str) -> String {
    Value::from(text).to_debug_string()
}

fn raise(message: String) -> FrostError {
    FrostError::from_string(message)
}

pub(crate) fn invalid_url(function: &str, url: &str, reason: impl std::fmt::Display) -> FrostError {
    raise(format!(
        "Function {function} got an invalid URL {}: {reason}",
        quoted(url)
    ))
}

/// A URL that is valid but cannot be sent; not quoted, as it may be huge.
pub(crate) fn unsendable_url(function: &str, reason: impl std::fmt::Display) -> FrostError {
    raise(format!("Function {function} cannot send its URL: {reason}"))
}

pub(crate) fn not_http(function: &str, url: &str) -> FrostError {
    raise(format!(
        "Function {function} requires an http or https URL, but got {}",
        quoted(url)
    ))
}

pub(crate) fn relative_without_base(function: &str, url: &str) -> FrostError {
    raise(format!(
        "Function {function} got the relative URL {}, but its client has no base_url",
        quoted(url)
    ))
}

pub(crate) fn base_url_with_extras(function: &str, url: &str) -> FrostError {
    raise(format!(
        "Function {function} requires a base_url without a query or fragment, but got {}",
        quoted(url)
    ))
}

pub(crate) fn invalid_host(
    function: &str,
    host: &str,
    reason: impl std::fmt::Display,
) -> FrostError {
    raise(format!(
        "Function {function} got an invalid host {}: {reason}",
        quoted(host)
    ))
}

pub(crate) fn invalid_method(function: &str, method: &str) -> FrostError {
    raise(format!(
        "Function {function} got an invalid method {}",
        quoted(method)
    ))
}

pub(crate) fn invalid_header_name(function: &str, name: &str) -> FrostError {
    raise(format!(
        "Function {function} got an invalid header name {}",
        quoted(name)
    ))
}

pub(crate) fn invalid_header_value(function: &str, name: &str, value: &str) -> FrostError {
    raise(format!(
        "Function {function} got an invalid value {} for header {}",
        quoted(value),
        quoted(name)
    ))
}

pub(crate) fn headers_differing_in_case(function: &str, first: &str, second: &str) -> FrostError {
    raise(format!(
        "Function {function} got headers {} and {}, which differ only in case",
        quoted(first),
        quoted(second)
    ))
}

pub(crate) fn managed_header(function: &str, name: &str) -> FrostError {
    raise(format!(
        "Function {function} cannot set header {}: the HTTP client manages it",
        quoted(name)
    ))
}

pub(crate) fn both_bodies(function: &str) -> FrostError {
    raise(format!(
        "Function {function} takes option `body` or `body_file`, not both"
    ))
}

pub(crate) fn cannot_read(
    function: &str,
    path: &str,
    reason: impl std::fmt::Display,
) -> FrostError {
    raise(format!(
        "Function {function} cannot read {}: {reason}",
        quoted(path)
    ))
}

pub(crate) fn cannot_save(
    function: &str,
    path: &str,
    reason: impl std::fmt::Display,
) -> FrostError {
    raise(format!(
        "Function {function} cannot save to {}: {reason}",
        quoted(path)
    ))
}

/// A `ca` entry, at `index`, that holds no usable certificate.
pub(crate) fn invalid_ca(function: &str, index: u64, reason: impl std::fmt::Display) -> FrostError {
    raise(format!(
        "Function {function} got an invalid certificate in `ca[{index}]`: {reason}"
    ))
}

pub(crate) fn invalid_proxy(
    function: &str,
    url: &str,
    reason: impl std::fmt::Display,
) -> FrostError {
    raise(format!(
        "Function {function} got an invalid proxy {}: {reason}",
        quoted(url)
    ))
}

pub(crate) fn no_concurrency(function: &str) -> FrostError {
    raise(format!(
        "Function {function} requires option `concurrency` to be at least 1"
    ))
}

pub(crate) fn colon_in_user(function: &str) -> FrostError {
    raise(format!(
        "Function {function} got a user containing `:`, which Basic authentication cannot send"
    ))
}

/// A failure building something reqwest checks itself, such as a client.
pub(crate) fn could_not(function: &str, what: &str, err: &reqwest::Error) -> FrostError {
    raise(format!(
        "Function {function} could not {what}: {}",
        with_causes(err)
    ))
}

/// `err`'s message followed by its causes': reqwest's own names only the
/// stage that failed, while its causes say why. A cause repeating the
/// message before it, as reqwest's nested errors can, is left out.
pub(crate) fn with_causes(err: &(dyn std::error::Error + 'static)) -> String {
    let mut messages: Vec<String> = std::iter::successors(Some(err), |&err| err.source())
        .map(ToString::to_string)
        .collect();
    messages.dedup();
    messages.join(": ")
}
