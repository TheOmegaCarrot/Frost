//! `http.client` and the client bundle it makes: a connection pool, and the
//! defaults its requests share.

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

use frostlang::native::{Bundle, De, Flat, Optional};
use frostlang::{FrostError, FrostResult, NativeCtx, Value, frost_bundle, frost_closure};
use reqwest::redirect::Policy;
use reqwest::{Certificate, Method, Proxy};
use url::Url;

use crate::HttpConfig;
use crate::config::{
    BatchOptions, ClientConfig, ProxySetting, RequestConfig, RequestOptions, UrlArg,
};
use crate::error;
use crate::exchange::{Prepared, exchange, outcome_value};
use crate::fields::{Headers, append_query};
use crate::files::{BodyFile, Destination};
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
        mut config: ClientConfig,
    ) -> Result<Self, FrostError> {
        let base_url = config
            .base_url
            .as_deref()
            .map(|base| check_base_url(function, base))
            .transpose()?;
        let headers = config
            .headers
            .take()
            .map(|fields| Headers::new(function, fields))
            .transpose()?
            .unwrap_or_default()
            .with_defaults(&Headers::user_agent());
        let certificates = ca_certificates(function, config.ca.as_deref().unwrap_or_default())?;
        let proxy = match &config.proxy {
            None => ProxyMode::Environment,
            Some(ProxySetting::Url(url)) => ProxyMode::Through(Box::new(proxy(function, url)?)),
            Some(ProxySetting::Direct) => ProxyMode::Direct,
        };
        let trust_system = config.system_ca != Some(false);
        let build = |trust_system| build_client(&config, &certificates, &proxy, trust_system);
        let http = match build(trust_system) {
            // reqwest refuses to make a client trusting only an empty system
            // store, rather than one trusting no certificate, which can still
            // make plain HTTP requests. Without verification, or with `ca`,
            // the system's store is not all a client trusts.
            Err(_)
                if trust_system
                    && config.verify_tls != Some(false)
                    && certificates.is_empty()
                    && system_store_is_empty() =>
            {
                build(false)
            }
            built => built,
        }
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
    ) -> Result<Prepared, FrostError> {
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
        let body_file = match (options.body, options.body_file) {
            (Some(_), Some(_)) => return Err(error::both_bodies(function)),
            (Some(body), None) => {
                request = request.body(body.as_bytes().to_vec());
                None
            }
            (None, Some(path)) => Some(BodyFile::new(function, &path)?),
            (None, None) => None,
        };
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
        let destination = options
            .save_to
            .map(|path| Destination::new(function, &path))
            .transpose()?;
        Ok(Prepared {
            request,
            body_file,
            destination,
        })
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
        let prepared = self.prepare(ctx.name(), url, method, options)?;
        let outcome = self.settings.runtime.run(
            ctx,
            exchange(self.http.clone(), prepared, self.settings.max_body_size),
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
            .map(move |prepared| exchange(http.clone(), prepared, max_body_size));
        let outcomes = self.settings.runtime.run_all(ctx, exchanges, concurrency)?;
        Ok(outcomes.into_iter().map(outcome_value).collect())
    }
}

/// Which proxy a client's requests go through.
enum ProxyMode {
    /// The proxies the environment names.
    Environment,
    Through(Box<Proxy>),
    /// None.
    Direct,
}

/// The reqwest client `config` describes, trusting `certificates`, and the
/// system's store too if `trust_system`, sending its requests by `proxy`.
fn build_client(
    config: &ClientConfig,
    certificates: &[Certificate],
    proxy: &ProxyMode,
    trust_system: bool,
) -> reqwest::Result<reqwest::Client> {
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
    if config.decompress == Some(false) {
        builder = builder.no_gzip().no_brotli().no_zstd();
    }
    let certificates = certificates.to_vec();
    builder = if trust_system {
        builder.tls_certs_merge(certificates)
    } else {
        builder.tls_certs_only(certificates)
    };
    if config.verify_tls == Some(false) {
        builder = builder.tls_danger_accept_invalid_certs(true);
    }
    builder = match proxy {
        ProxyMode::Environment => builder,
        ProxyMode::Through(proxy) => builder.proxy(Proxy::clone(proxy)),
        ProxyMode::Direct => builder.no_proxy(),
    };
    builder.build()
}

/// The certificates in `ca`, each entry PEM content holding one or more, for
/// the function `function`.
fn ca_certificates(function: &str, ca: &[Flat]) -> Result<Vec<Certificate>, FrostError> {
    let mut certificates = Vec::new();
    for (index, pem) in (0_u64..).zip(ca) {
        let found = Certificate::from_pem_bundle(pem.as_bytes())
            .map_err(|err| error::invalid_ca(function, index, error::with_causes(&err)))?;
        if found.is_empty() {
            return Err(error::invalid_ca(
                function,
                index,
                "no PEM certificate found",
            ));
        }
        certificates.extend(found);
    }
    Ok(certificates)
}

/// Whether the system's trust store holds no certificate, and could be read,
/// where reqwest reads it from the system's files: the store `SSL_CERT_FILE`
/// and `SSL_CERT_DIR` name, or else the distribution's. A store that cannot
/// be read is a misconfiguration, not an empty store.
#[cfg(all(
    unix,
    not(target_os = "android"),
    not(target_vendor = "apple"),
    not(target_arch = "wasm32"),
))]
fn system_store_is_empty() -> bool {
    let store = rustls_native_certs::load_native_certs();
    store.certs.is_empty() && store.errors.is_empty()
}

/// Whether the system's trust store holds no certificate: never, where the
/// operating system verifies certificates itself.
#[cfg(not(all(
    unix,
    not(target_os = "android"),
    not(target_vendor = "apple"),
    not(target_arch = "wasm32"),
)))]
fn system_store_is_empty() -> bool {
    false
}

/// The proxy at `url`, for the function `function`: an http or https proxy,
/// taken as http if `url` has no scheme.
fn proxy(function: &str, url: &str) -> Result<Proxy, FrostError> {
    // reqwest takes a URL with a host as it is, whatever its scheme, while it
    // is built to connect through http and https proxies only.
    if let Ok(parsed) = Url::parse(url)
        && parsed.has_host()
        && !matches!(parsed.scheme(), "http" | "https")
    {
        return Err(error::invalid_proxy(
            function,
            url,
            "its scheme must be http or https",
        ));
    }
    Proxy::all(url).map_err(|err| error::invalid_proxy(function, url, error::with_causes(&err)))
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
