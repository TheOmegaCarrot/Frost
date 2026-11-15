//! One request and its response, as run on the async runtime, and the
//! result Map a script gets.

use std::error::Error;

use frostlang::Value;
use reqwest::header::{CONTENT_LENGTH, HeaderMap, HeaderValue};
use url::Url;

use crate::error;
use crate::fields::{headers_value, text_or_bytes};
use crate::files::{BodyFile, Destination, Partial, ReadFailure};

/// A request ready to send: checked, with any files it uses still to open.
pub(crate) struct Prepared {
    pub(crate) request: reqwest::Request,
    /// The file to stream the request's body from.
    pub(crate) body_file: Option<BodyFile>,
    /// Where to save the response's body, rather than hold it.
    pub(crate) destination: Option<Destination>,
}

/// How a request went: a response, or why there is none.
pub(crate) type Outcome = Result<Response, Failure>;

/// A response, its body read whole.
pub(crate) struct Response {
    code: u16,
    headers: HeaderMap,
    body: ResponseBody,
    /// The final URL, after redirects.
    url: Url,
}

/// Where a response's body went.
enum ResponseBody {
    Held(Vec<u8>),
    /// Saved to the file at this absolute path.
    Saved(String),
}

/// Why a request failed once sent, rather than raising when prepared.
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
    Save,
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
            Self::Save => "save",
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
        Self {
            kind,
            message: error::with_causes(err),
        }
    }

    fn too_large(max_body_size: u64) -> Self {
        Self {
            kind: FailureKind::TooLarge,
            message: format!(
                "the response body is larger than the maximum of {max_body_size} bytes"
            ),
        }
    }

    /// A failure to read the body file at `path`.
    fn read(path: &str, err: &std::io::Error) -> Self {
        Self {
            kind: FailureKind::Body,
            message: format!("could not read {}: {err}", error::quoted(path)),
        }
    }

    /// A failure to save to the file at `path`.
    fn save(path: &str, err: &std::io::Error) -> Self {
        Self {
            kind: FailureKind::Save,
            message: format!("could not save to {}: {err}", error::quoted(path)),
        }
    }

    /// Why sending a request failed, given `err` as reqwest reports it: as
    /// its body file's `read_failure` says, if reading that failed, since
    /// `err` need not say so.
    fn sending(err: &reqwest::Error, read_failure: Option<&ReadFailure>) -> Self {
        match read_failure.and_then(ReadFailure::get) {
            Some((path, read_err)) => Self::read(path, read_err),
            None => Self::from_reqwest(err),
        }
    }
}

/// Sends `prepared` with `http`, and reads the response's body whole: into
/// memory, failing if it is larger than `max_body_size` bytes, or into its
/// destination.
pub(crate) async fn exchange(
    http: reqwest::Client,
    prepared: Prepared,
    max_body_size: Option<u64>,
) -> Outcome {
    let Prepared {
        mut request,
        body_file,
        destination,
    } = prepared;
    let read_failure = match &body_file {
        Some(file) => Some(attach_body_file(&mut request, file).await?),
        None => None,
    };
    // Before sending, so that a destination that cannot be written fails
    // without the request having any effect.
    let partial = match &destination {
        Some(destination) => Some(
            destination
                .create()
                .await
                .map_err(|err| Failure::save(destination.path(), &err))?,
        ),
        None => None,
    };
    let mut response = http
        .execute(request)
        .await
        .map_err(|err| Failure::sending(&err, read_failure.as_ref()))?;
    let headers = std::mem::take(response.headers_mut());
    let (code, url) = (response.status().as_u16(), response.url().clone());
    let body = match partial {
        Some(partial) => ResponseBody::Saved(save(&mut response, partial).await?),
        None => ResponseBody::Held(hold(&mut response, max_body_size).await?),
    };
    Ok(Response {
        code,
        headers,
        body,
        url,
    })
}

/// The body of `response`, read into memory, failing if it is larger than
/// `max_body_size` bytes.
async fn hold(
    response: &mut reqwest::Response,
    max_body_size: Option<u64>,
) -> Result<Vec<u8>, Failure> {
    // The maximum, if `size` exceeds it.
    let exceeded_max = |size: u64| max_body_size.filter(|&max| size > max);
    // A declared length too large fails before reading any of the body.
    if let Some(max) = response.content_length().and_then(exceeded_max) {
        return Err(Failure::too_large(max));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|err| Failure::from_reqwest(&err))?
    {
        let size = u64::try_from(body.len() + chunk.len()).unwrap_or(u64::MAX);
        if let Some(max) = exceeded_max(size) {
            return Err(Failure::too_large(max));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Saves the body of `response` through `partial` to its destination, giving
/// the destination's absolute path. Any failure, or dropping this before it
/// finishes, deletes the partial file and leaves the destination as it was.
async fn save(response: &mut reqwest::Response, mut partial: Partial) -> Result<String, Failure> {
    let path = partial.path().to_owned();
    let fail = |err: std::io::Error| Failure::save(&path, &err);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|err| Failure::from_reqwest(&err))?
    {
        partial.write(&chunk).await.map_err(fail)?;
    }
    partial.finish().await.map_err(fail)?;
    Ok(path)
}

/// Attaches `file` to `request` as its body, giving where a failure reading
/// it while it is sent is recorded.
async fn attach_body_file(
    request: &mut reqwest::Request,
    file: &BodyFile,
) -> Result<ReadFailure, Failure> {
    let upload = file
        .open()
        .await
        .map_err(|err| Failure::read(file.path(), &err))?;
    // A streamed body would otherwise go chunked, which some servers refuse.
    request
        .headers_mut()
        .insert(CONTENT_LENGTH, HeaderValue::from(upload.length));
    // A streamed body cannot be sent twice, so reqwest does not follow a
    // redirect that would resend it, and the redirect is the response, as the
    // extension's documentation promises.
    *request.body_mut() = Some(upload.body);
    Ok(upload.failure)
}

/// The result Map a script gets for `outcome`.
pub(crate) fn outcome_value(outcome: Outcome) -> Value {
    match outcome {
        Ok(response) => {
            let body = match response.body {
                ResponseBody::Held(body) => ("body", text_or_bytes(body)),
                ResponseBody::Saved(path) => ("file", Value::from(path)),
            };
            Value::map([
                ("ok", Value::Bool(true)),
                (
                    "response",
                    Value::map([
                        ("code", Value::Int(i64::from(response.code))),
                        ("headers", headers_value(&response.headers)),
                        body,
                        ("url", Value::from(response.url.as_str())),
                    ]),
                ),
            ])
        }
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
