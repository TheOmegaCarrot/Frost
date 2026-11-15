//! HTTP requests for Frost scripts.
//!
//! A host installs the extension with [`extension`], giving it a
//! [`Runtime`] to run requests on, and scripts import it as `ext.http`:
//!
//! ```
//! use frostlang::ImporterBuilder;
//! use frostlang_http::HttpConfig;
//! use frostlang_tokio::Runtime;
//!
//! let importer = ImporterBuilder::new()
//!     .with_extension(frostlang_http::extension(HttpConfig::new(Runtime::new())))
//!     .expect("no other extension is named `http`")
//!     .build();
//! ```
//!
//! The extension is built on [`reqwest`], over TLS from `rustls`, which
//! verifies certificates against the operating system's trust store.
//! Building it needs a C compiler.

mod auth;
mod client;
mod config;
mod error;
mod exchange;
mod fields;
mod files;
mod urls;

use std::sync::Arc;

use frostlang::Extension;
use frostlang_tokio::Runtime;

/// The host's settings for the `ext.http` extension.
#[derive(Debug, Clone)]
pub struct HttpConfig {
    runtime: Runtime,
    max_body_size: Option<u64>,
}

impl HttpConfig {
    /// Settings running every request on `runtime`, and holding response
    /// bodies of any size.
    pub fn new(runtime: Runtime) -> Self {
        Self {
            runtime,
            max_body_size: None,
        }
    }

    /// Fail a request, as `too_large`, once its response body exceeds `bytes`
    /// bytes, after any decompression.
    pub fn with_max_body_size(mut self, bytes: u64) -> Self {
        self.max_body_size = Some(bytes);
        self
    }
}

/// The `ext.http` extension, running under `config`.
///
/// Scripts make requests through a client:
///
/// ```frost
/// def http = import('ext.http')
/// def api = http.client({base_url: 'https://api.example.com/v1'})
/// def result = api.get('users', {query: {page: 2}})
/// if result.ok: result.response.body
/// else: result.error.message
/// ```
///
/// Durations are Int microseconds. A function raises only when it cannot
/// make a well-formed request from what it was given; once a request reaches
/// for the network, it returns a result, even on failure. There are no JSON
/// helpers: `ext.formats` encodes request bodies and decodes response bodies.
///
/// # The module
///
/// - `client(config?)` makes a client. Its connections are reused for as long
///   as the client is in use.
/// - `parse_url(url)` gives the URL Map of an http or https URL's text, or
///   Null for any other text: `{host, path, tls}`, with `port`, `query`,
///   `user`, `password`, and `fragment` if the URL has them. Each part but
///   the host is percent-encoded, as in the URL's text.
/// - `build_url(map)` gives the text of the URL a URL Map describes; `host`,
///   `path`, and `tls` are required. Characters a part cannot hold are
///   percent-encoded.
/// - `form(fields)` gives `fields` as a URL-encoded form body, with the value
///   rules of a request's `query`; set the `content-type` header to
///   `application/x-www-form-urlencoded` to send it.
/// - `auth.bearer(token)` and `auth.basic(user, password)` give an
///   `authorization` header, as a Map to combine with other headers by `+`.
///
/// # A client's config
///
/// Every key is optional.
///
/// - `base_url`: a URL that relative request URLs join onto. The join is
///   plain, as for a directory: the base with a trailing `/` ensured, then the
///   relative URL with any leading `/` removed. So `users` and `/users` both
///   join `https://a.example/v1` as `https://a.example/v1/users`. A URL with a
///   scheme ignores the base.
/// - `headers`: headers every request sends, unless it sends a header of the
///   same name itself. Unless they name it, a client sends `user-agent` as
///   `frost-http/` and the extension's version.
/// - `timeout`: a limit on each whole request, response body included. None
///   by default.
/// - `connect_timeout`: a limit on making a connection; 10 seconds by default.
/// - `read_timeout`: a limit on waiting; 30 seconds by default. Until the
///   response starts, it limits the whole wait, sending the body included;
///   after that, each wait for more of the response's body. A longer upload
///   needs a longer `read_timeout`; since it is a client's option, that may
///   call for a client of its own.
/// - `redirects`: the most redirects a request follows; 10 by default. With
///   `0`, a redirect is the response.
/// - `decompress`: `true` by default. With `false`, the client asks for no
///   compression, and a body a server compresses anyway, as some do a `.gz`
///   download, is given as sent, its headers intact.
/// - `verify_tls`: `true` by default. With `false`, any server certificate is
///   accepted, and `ca` and `system_ca` go unused.
/// - `ca`: an Array of certificate authorities to trust, each PEM content as a
///   String or Bytes, which may hold several certificates. None by default.
/// - `system_ca`: whether the operating system's trusted authorities are
///   trusted too; `true` by default. With `false` and no `ca`, or a system
///   that has no trusted authorities, no server certificate is trusted.
/// - `proxy`: the URL of an http or https proxy to send every request
///   through, taken as http if it has no scheme, or `false` for none. By
///   default, the proxies the environment names, as in `http_proxy`, for
///   loopback addresses too unless `no_proxy` names them.
///
/// # A client's functions
///
/// - `request(config)` sends any request.
/// - `get(url, options?)`, and likewise `head`, `delete`, `post`, `put`, and
///   `patch`, send a request with that method. `options` is a request's
///   config without `url` and `method`.
/// - `request_all(configs, options?)` sends a request for each config, and
///   gives their results in the same order. `options` may give
///   `concurrency`, how many requests are in flight at once: 8 by default,
///   and `1` sends them one at a time. If any config cannot make a request,
///   it raises before sending any.
///
/// A request's config:
///
/// - `url`, required: the URL's text, or a URL Map. A relative URL needs the
///   client's `base_url`. A user and password in the URL are sent as Basic
///   authorization, unless the request sends an `authorization` header. A
///   fragment is never sent.
/// - `method`: `'GET'` by default. A standard method, in any case, is sent
///   uppercase; any other is sent exactly as written.
/// - `query`: parameters to add to the URL. Each value is a String, Int,
///   Bool, Null for a parameter without a value, or an Array of them to
///   repeat the parameter.
/// - `headers`: each value a String, or an Array of String to repeat the
///   header. Names may be in any case, but two names may not differ only in
///   case. A name with an empty Array sends none of the client's header of
///   that name. The client sets `host`, `content-length`,
///   `transfer-encoding`, and `accept-encoding` itself, in a client's
///   `headers` too, so naming one raises.
/// - `body`: a String or Bytes, sent with any method.
/// - `body_file`: the path of a regular file to send as the body, read as it
///   is sent. It is sent at its length when its request is sent: if it grows
///   while sent, only that much is sent, and if it is cut short, the request
///   fails. At most one of `body` and `body_file`. A redirect that would
///   send the body again (307 or 308, or 301 or 302 for any method but POST)
///   is not followed, since a file is sent only once: the redirect is the
///   response.
/// - `timeout`: replaces the client's `timeout` for this request.
/// - `save_to`: the path of a file to save the response's body to, rather
///   than give it. The body is written beside the file as it arrives, and
///   replaces the file only once it is whole: a request that fails or is
///   cancelled leaves the file as it was. The directory must exist. The body
///   has no maximum size. Any response's body is saved, an error page
///   included, so to keep a file from being replaced by one, save to a new
///   path and move the file once `code` is as expected.
///
/// A request's files are opened only as it is sent, so a batch holds files
/// open only for the requests in flight. A file that cannot be opened then
/// fails the request, rather than raising.
///
/// # Results
///
/// A request that got a response, whatever its status, gives
/// `{ok: true, response: {code, headers, body, url}}`:
///
/// - `code`: the status code.
/// - `headers`: a Map from each lowercase name to its value, or to an Array of
///   its values if it is repeated.
/// - `body`: a String if it is UTF-8, otherwise Bytes, whatever its
///   `content-type`.
/// - `file`, in place of `body` with `save_to`: the absolute path of the file
///   saved.
/// - `url`: the final URL, after any redirects.
///
/// A header's value is likewise a String if it is UTF-8, otherwise Bytes.
/// Unless the client's `decompress` is `false`, compression is the client's
/// business: a compressed body is decompressed, and the headers describe it
/// as decompressed.
///
/// A request that failed gives `{ok: false, error: {kind, message}}`, where
/// `kind` is one of:
///
/// - `'timeout'`: a timeout passed.
/// - `'connect'`: no connection could be made, as when the host is unknown,
///   refuses connections, or fails TLS verification.
/// - `'redirect'`: more redirects than the client follows, or a redirect
///   loop.
/// - `'body'`: a body could not be sent or read whole, as when the
///   connection closed partway, or a `body_file` could not be read.
/// - `'decode'`: the response body could not be decompressed.
/// - `'request'`: the request failed otherwise.
/// - `'save'`: the response body could not be saved to its `save_to` file,
///   as when its partial file could not be made.
/// - `'too_large'`: the response body was larger than the host allows.
///
/// A cancelled run stops every request it is waiting for.
pub fn extension(config: HttpConfig) -> Extension {
    let config = Arc::new(config);
    Extension::builder("http")
        .value("client", client::client_function(config))
        .function(urls::parse_url_frost())
        .function(urls::build_url_frost())
        .function(fields::form_frost())
        .module("auth", |auth| {
            auth.function(auth::bearer_frost())
                .function(auth::basic_frost())
        })
        .build()
}
