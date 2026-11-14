//! `http.client` and the client bundle it makes: a connection pool, and the
//! defaults its requests share.

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

use frostlang::native::{Bundle, De, Optional};
use frostlang::{FrostError, FrostResult, NativeCtx, Value, frost_bundle, frost_closure};
use reqwest::Method;
use reqwest::redirect::Policy;

use crate::HttpConfig;
use crate::config::{BatchOptions, ClientConfig, RequestConfig, RequestOptions, UrlArg};
use crate::error;
use crate::exchange::{exchange, outcome_value};
use crate::fields::{Headers, append_query};
use crate::urls::{check_base_url, request_url};

/// How long a connection may take to establish, unless a client says.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// How long a response may go without sending data, unless a client says.
const READ_TIMEOUT: Duration = Duration::from_secs(30);
/// How many redirects a request follows, unless its client says.
const REDIRECTS: u64 = 10;
/// How many requests of a batch are in flight at once, unless it says.
const CONCURRENCY: u64 = 8;

/// The name of the function `http.client`.
const CLIENT: &str = "ext.http.client";

/// The function `http.client`, making clients under the host's `settings`.
pub(crate) fn client_function(settings: Arc<HttpConfig>) -> Value {
    frost_closure!(CLIENT, move |config: Optional<De<ClientConfig>>| {
        let Optional(config) = config;
        let config = config.map_or_else(ClientConfig::default, |De(config)| config);
        let client = Client::new(CLIENT, Arc::clone(&settings), config)?;
        FrostResult::Ok(Bundle::new(client).with(Client::frost_functions).build())
    })
}

/// A client: its connection pool, and the defaults its requests share.
pub(crate) struct Client {
    http: reqwest::Client,
    /// The base relative URLs join onto, ending in `/`.
    base_url: Option<String>,
    /// Headers every request sends, unless it names them itself.
    headers: Headers,
    /// The host's settings for the extension.
    settings: Arc<HttpConfig>,
}

impl Client {
    /// A client as `config` describes, for the function `function`.
    fn new(
        function: &str,
        settings: Arc<HttpConfig>,
        config: ClientConfig,
    ) -> Result<Self, FrostError> {
        let base_url = config
            .base_url
            .map(|base| check_base_url(function, &base))
            .transpose()?;
        let headers = config
            .headers
            .map(|fields| Headers::new(function, fields))
            .transpose()?
            .unwrap_or_default()
            .with_defaults(&Headers::user_agent());
        let redirects = match config.redirects.unwrap_or(REDIRECTS) {
            0 => Policy::none(),
            most => Policy::limited(usize::try_from(most).unwrap_or(usize::MAX)),
        };
        let mut builder = reqwest::Client::builder()
            .connect_timeout(
                config
                    .connect_timeout
                    .map_or(CONNECT_TIMEOUT, Duration::from_micros),
            )
            .read_timeout(
                config
                    .read_timeout
                    .map_or(READ_TIMEOUT, Duration::from_micros),
            )
            .redirect(redirects);
        if let Some(timeout) = config.timeout {
            builder = builder.timeout(Duration::from_micros(timeout));
        }
        let http = builder
            .build()
            .map_err(|err| error::could_not(function, "create a client", &err))?;
        Ok(Self {
            http,
            base_url,
            headers,
            settings,
        })
    }

    /// The request that `url`, `method`, and `options` describe, checked, for
    /// the function `function`.
    fn prepare(
        &self,
        function: &str,
        url: &UrlArg,
        method: Option<&str>,
        options: RequestOptions,
    ) -> Result<reqwest::Request, FrostError> {
        let mut url = request_url(function, self.base_url.as_deref(), url)?;
        if let Some(query) = &options.query {
            append_query(&mut url, query);
        }
        let method = method.map_or(Ok(Method::GET), |method| parse_method(function, method))?;
        let headers = options
            .headers
            .map(|fields| Headers::new(function, fields))
            .transpose()?
            .unwrap_or_default();
        let mut request = self
            .http
            .request(method, url)
            .headers(headers.with_defaults(&self.headers).to_map());
        if let Some(body) = options.body {
            request = request.body(body.as_bytes().to_vec());
        }
        if let Some(timeout) = options.timeout {
            request = request.timeout(Duration::from_micros(timeout));
        }
        let request = request
            .build()
            .map_err(|err| error::could_not(function, "form the request", &err))?;
        // reqwest converts the URL for sending only once it sends, and some
        // URLs, such as one too long, do not convert.
        request
            .url()
            .as_str()
            .parse::<hyper::Uri>()
            .map_err(|reason| error::unsendable_url(function, reason))?;
        Ok(request)
    }

    /// Sends the request `url`, `method`, and `options` describe, for the
    /// function `ctx` runs.
    fn send(
        &self,
        ctx: &NativeCtx<'_>,
        url: &UrlArg,
        method: Option<&str>,
        options: RequestOptions,
    ) -> FrostResult {
        let request = self.prepare(ctx.name(), url, method, options)?;
        let outcome = self.settings.runtime.run(
            ctx,
            exchange(self.http.clone(), request, self.settings.max_body_size),
        )?;
        Ok(outcome_value(outcome))
    }

    /// Sends a request with `method`, as `get` and its kin do.
    fn send_with(
        &self,
        ctx: &NativeCtx<'_>,
        method: &str,
        url: De<UrlArg>,
        options: Optional<De<RequestOptions>>,
    ) -> FrostResult {
        let Optional(options) = options;
        let options = options.map_or_else(RequestOptions::default, |De(options)| options);
        self.send(ctx, &url.0, Some(method), options)
    }
}

#[frost_bundle(prefix = "client")]
impl Client {
    fn request(
        &self,
        #[frost(ctx)] ctx: &mut NativeCtx<'_>,
        config: De<RequestConfig>,
    ) -> FrostResult {
        let (url, method, options) = config.0.into_parts();
        self.send(ctx, &url, method.as_deref(), options)
    }

    fn get(
        &self,
        #[frost(ctx)] ctx: &mut NativeCtx<'_>,
        url: De<UrlArg>,
        options: Optional<De<RequestOptions>>,
    ) -> FrostResult {
        self.send_with(ctx, "GET", url, options)
    }

    fn head(
        &self,
        #[frost(ctx)] ctx: &mut NativeCtx<'_>,
        url: De<UrlArg>,
        options: Optional<De<RequestOptions>>,
    ) -> FrostResult {
        self.send_with(ctx, "HEAD", url, options)
    }

    fn delete(
        &self,
        #[frost(ctx)] ctx: &mut NativeCtx<'_>,
        url: De<UrlArg>,
        options: Optional<De<RequestOptions>>,
    ) -> FrostResult {
        self.send_with(ctx, "DELETE", url, options)
    }

    fn post(
        &self,
        #[frost(ctx)] ctx: &mut NativeCtx<'_>,
        url: De<UrlArg>,
        options: Optional<De<RequestOptions>>,
    ) -> FrostResult {
        self.send_with(ctx, "POST", url, options)
    }

    fn put(
        &self,
        #[frost(ctx)] ctx: &mut NativeCtx<'_>,
        url: De<UrlArg>,
        options: Optional<De<RequestOptions>>,
    ) -> FrostResult {
        self.send_with(ctx, "PUT", url, options)
    }

    fn patch(
        &self,
        #[frost(ctx)] ctx: &mut NativeCtx<'_>,
        url: De<UrlArg>,
        options: Optional<De<RequestOptions>>,
    ) -> FrostResult {
        self.send_with(ctx, "PATCH", url, options)
    }

    fn request_all(
        &self,
        #[frost(ctx)] ctx: &mut NativeCtx<'_>,
        configs: Vec<De<RequestConfig>>,
        options: Optional<De<BatchOptions>>,
    ) -> FrostResult {
        let function = ctx.name();
        let Optional(options) = options;
        let concurrency = options
            .and_then(|De(options)| options.concurrency)
            .unwrap_or(CONCURRENCY);
        let concurrency = NonZeroUsize::new(usize::try_from(concurrency).unwrap_or(usize::MAX))
            .ok_or_else(|| error::no_concurrency(function))?;
        // Every request is checked before any is sent.
        let requests = configs
            .into_iter()
            .map(|De(config)| {
                let (url, method, options) = config.into_parts();
                self.prepare(function, &url, method.as_deref(), options)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let http = self.http.clone();
        let max_body_size = self.settings.max_body_size;
        let exchanges = requests
            .into_iter()
            .map(move |request| exchange(http.clone(), request, max_body_size));
        let outcomes = self.settings.runtime.run_all(ctx, exchanges, concurrency)?;
        Ok(outcomes.into_iter().map(outcome_value).collect())
    }
}

/// The method `method` names, for the function `function`: a standard method
/// in any case, as uppercase, or any other valid token exactly as written.
fn parse_method(function: &str, method: &str) -> Result<Method, FrostError> {
    const STANDARD: [Method; 9] = [
        Method::GET,
        Method::HEAD,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::PATCH,
        Method::OPTIONS,
        Method::TRACE,
        Method::CONNECT,
    ];
    if let Some(standard) = STANDARD
        .iter()
        .find(|standard| standard.as_str().eq_ignore_ascii_case(method))
    {
        return Ok(standard.clone());
    }
    Method::from_bytes(method.as_bytes()).map_err(|_| error::invalid_method(function, method))
}
