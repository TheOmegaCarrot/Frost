//! One request and its response, as run on the async runtime, and the
//! result Map a script gets.

use std::error::Error;

use frostlang::Value;
use reqwest::header::HeaderMap;
use url::Url;

use crate::fields::{headers_value, text_or_bytes};

/// How a request went: a response, or why there is none.
pub(crate) type Outcome = Result<BufferedResponse, Failure>;

/// A response, its body read whole.
pub(crate) struct BufferedResponse {
    code: u16,
    headers: HeaderMap,
    body: Vec<u8>,
    /// The final URL, after redirects.
    url: Url,
}

/// Why an exchange failed after reaching for the network.
pub(crate) struct Failure {
    kind: FailureKind,
    message: String,
}

/// A failure's `kind`, as scripts see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailureKind {
    Timeout,
    Connect,
    Redirect,
    Body,
    Decode,
    Request,
    TooLarge,
}

impl FailureKind {
    fn name(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::Connect => "connect",
            Self::Redirect => "redirect",
            Self::Body => "body",
            Self::Decode => "decode",
            Self::Request => "request",
            Self::TooLarge => "too_large",
        }
    }
}

impl Failure {
    fn from_reqwest(err: &reqwest::Error) -> Self {
        let causes = || std::iter::successors(err.source(), |&cause| cause.source());
        // Most specific first: a timeout connecting is a timeout.
        let kind = if err.is_timeout() {
            FailureKind::Timeout
        } else if err.is_redirect() {
            FailureKind::Redirect
        } else if err.is_connect() {
            FailureKind::Connect
        } else if err.is_decode() {
            // reqwest reports any failure reading a response's body as
            // decoding it; only the cause tells a connection failing apart.
            if causes().any(<dyn Error>::is::<hyper::Error>) {
                FailureKind::Body
            } else {
                FailureKind::Decode
            }
        } else if err.is_body() {
            FailureKind::Body
        } else {
            FailureKind::Request
        };
        // reqwest's own message names only the stage that failed; its causes
        // say why.
        let message = std::iter::once(err.to_string())
            .chain(causes().map(ToString::to_string))
            .collect::<Vec<_>>()
            .join(": ");
        Self { kind, message }
    }

    fn too_large(max_body_size: u64) -> Self {
        Self {
            kind: FailureKind::TooLarge,
            message: format!(
                "the response body is larger than the maximum of {max_body_size} bytes"
            ),
        }
    }
}

/// Sends `request` with `http`, and reads the response's body whole, failing
/// if it is larger than `max_body_size` bytes.
pub(crate) async fn exchange(
    http: reqwest::Client,
    request: reqwest::Request,
    max_body_size: Option<u64>,
) -> Outcome {
    let fail = |err| Failure::from_reqwest(&err);
    let mut response = http.execute(request).await.map_err(fail)?;
    // The maximum, if `size` exceeds it.
    let exceeded_max = |size: u64| max_body_size.filter(|&max| size > max);
    // A declared length too large fails before reading any of the body.
    if let Some(max) = response.content_length().and_then(exceeded_max) {
        return Err(Failure::too_large(max));
    }
    let mut buffered = BufferedResponse {
        code: response.status().as_u16(),
        headers: std::mem::take(response.headers_mut()),
        body: Vec::new(),
        url: response.url().clone(),
    };
    while let Some(chunk) = response.chunk().await.map_err(fail)? {
        let size = u64::try_from(buffered.body.len() + chunk.len()).unwrap_or(u64::MAX);
        if let Some(max) = exceeded_max(size) {
            return Err(Failure::too_large(max));
        }
        buffered.body.extend_from_slice(&chunk);
    }
    Ok(buffered)
}

/// The result Map a script gets for `outcome`.
pub(crate) fn outcome_value(outcome: Outcome) -> Value {
    match outcome {
        Ok(buffered) => Value::map([
            ("ok", Value::Bool(true)),
            (
                "response",
                Value::map([
                    ("code", Value::Int(i64::from(buffered.code))),
                    ("headers", headers_value(&buffered.headers)),
                    ("body", text_or_bytes(buffered.body)),
                    ("url", Value::from(buffered.url.as_str())),
                ]),
            ),
        ]),
        Err(failure) => Value::map([
            ("ok", Value::Bool(false)),
            (
                "error",
                Value::map([
                    ("kind", Value::from(failure.kind.name())),
                    ("message", Value::from(failure.message)),
                ]),
            ),
        ]),
    }
}
