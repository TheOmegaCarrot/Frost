//! `ext.http`'s use of the system's trust store, which `frost`'s child process
//! is given through `SSL_CERT_FILE`. The variable replaces the system's
//! certificate authorities on Unix systems other than Apple's and Android,
//! where the system's store is read from files; this module is built only
//! there.

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair,
};
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use tempfile::TempDir;

use crate::extensions::{answer_once, eval_with, without_proxies};

/// A certificate authority made for a test.
struct TestCa {
    issuer: CertifiedIssuer<'static, KeyPair>,
}

impl TestCa {
    fn new(name: &str) -> Self {
        let mut params = CertificateParams::new(Vec::<String>::new()).expect("no names are valid");
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.distinguished_name.push(DnType::CommonName, name);
        let key = KeyPair::generate().expect("a key can be made");
        let issuer = CertifiedIssuer::self_signed(params, key).expect("an authority signs itself");
        Self { issuer }
    }

    /// The authority's certificate, as a Frost String literal.
    fn pem_literal(&self) -> String {
        format!("'{}'", self.issuer.pem().trim_end().replace('\n', "\\n"))
    }

    /// A server's TLS settings, presenting a certificate this authority
    /// signed for 127.0.0.1.
    fn server_config(&self) -> Arc<rustls::ServerConfig> {
        let mut params =
            CertificateParams::new(vec![String::from("127.0.0.1")]).expect("the name is valid");
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let key = KeyPair::generate().expect("a key can be made");
        let certificate = params
            .signed_by(&key, &self.issuer)
            .expect("the authority signs the certificate");
        let private_key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der()));
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let config = rustls::ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("the default protocols are supported")
            .with_no_client_auth()
            .with_single_cert(vec![certificate.der().clone()], private_key)
            .expect("the certificate matches its key");
        Arc::new(config)
    }
}

/// An HTTPS server on the loopback interface with a certificate `ca` signed,
/// answering one connection's request with `secure`, if its client trusts
/// the certificate.
fn https_once(ca: &TestCa) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port is free");
    let address = listener
        .local_addr()
        .expect("a bound socket has an address");
    let config = ca.server_config();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("the script connects");
        let session = rustls::ServerConnection::new(config).expect("a TLS session starts");
        let mut stream = BufReader::new(rustls::StreamOwned::new(session, stream));
        // A client that does not trust the certificate ends the handshake,
        // and with it the reading.
        let mut line = String::new();
        while stream.read_line(&mut line).is_ok_and(|read| read > 0) {
            if line == "\r\n" {
                let _ = stream.get_mut().write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-length: 6\r\nconnection: close\r\n\r\nsecure",
                );
                break;
            }
            line.clear();
        }
    });
    address
}

/// A system trust store for `frost`'s child process: a PEM file, in a
/// scratch directory deleted with it.
struct SystemStore {
    dir: TempDir,
}

impl SystemStore {
    /// A store holding `content`, as PEM.
    fn holding(content: &str) -> Self {
        let dir = tempfile::tempdir().expect("a scratch directory can be made");
        std::fs::write(dir.path().join("system.pem"), content).expect("the store is written");
        Self { dir }
    }

    /// A store trusting only `ca`.
    fn trusting(ca: &TestCa) -> Self {
        Self::holding(&ca.issuer.pem())
    }

    fn path(&self) -> PathBuf {
        self.dir.path().join("system.pem")
    }

    /// A `frost` command with this as its system trust store.
    fn command(&self) -> Command {
        let mut command = without_proxies();
        command
            .env("SSL_CERT_FILE", self.path())
            .env_remove("SSL_CERT_DIR");
        command
    }
}

/// What a GET of `url` by a client with `config` gives, run with `store` as
/// the system's trust store: the body, or the failure's kind.
fn get_url(url: &str, config: &str, store: &SystemStore) -> String {
    let code = format!(
        r"
        def http = import('ext.http')
        def result = http.client({config}).get('{url}')
        print(if result.ok: result.response.body else: result.error.kind)
        "
    );
    eval_with(&code, store.command()).trim_end().to_string()
}

/// What a GET of the HTTPS server at `server` gives, as [`get_url`].
fn get(server: SocketAddr, config: &str, store: &SystemStore) -> String {
    get_url(&format!("https://{server}/"), config, store)
}

#[test]
fn http_trusts_the_systems_authorities_by_default() {
    let ca = TestCa::new("system");
    let store = SystemStore::trusting(&ca);
    assert_eq!(get(https_once(&ca), "", &store), "secure");
    assert_eq!(get(https_once(&ca), "{system_ca: true}", &store), "secure");
    assert_eq!(
        get(https_once(&ca), "{system_ca: false}", &store),
        "connect",
        "without the system's authorities, nothing is trusted"
    );
}

#[test]
fn http_extra_authorities_join_the_systems_unless_it_is_left_out() {
    let system = TestCa::new("system");
    let extra = TestCa::new("extra");
    let store = SystemStore::trusting(&system);
    let config = format!("{{ca: [{}]}}", extra.pem_literal());
    assert_eq!(
        get(https_once(&system), &config, &store),
        "secure",
        "the system's authority, beside an extra one"
    );
    assert_eq!(
        get(https_once(&extra), &config, &store),
        "secure",
        "the extra authority, beside the system's"
    );
    let config = format!("{{ca: [{}], system_ca: false}}", extra.pem_literal());
    assert_eq!(
        get(https_once(&system), &config, &store),
        "connect",
        "the system's authority, left out"
    );
    assert_eq!(get(https_once(&extra), &config, &store), "secure");
}

#[test]
fn http_with_an_empty_system_store_trusts_only_extra_authorities() {
    let ca = TestCa::new("extra");
    let store = SystemStore::holding("");
    // The client is made, though it trusts no authority.
    assert_eq!(
        get(https_once(&ca), "", &store),
        "connect",
        "no authority is trusted"
    );
    assert_eq!(
        get(
            https_once(&ca),
            &format!("{{ca: [{}]}}", ca.pem_literal()),
            &store
        ),
        "secure",
        "an extra authority is"
    );
    let (address, server) = answer_once("hello");
    assert_eq!(
        get_url(&format!("http://{address}/"), "", &store),
        "hello",
        "plain HTTP needs no authority"
    );
    server.join().expect("the server answered");
}

#[test]
fn http_with_an_unusable_system_store_cannot_make_a_client() {
    // PEM holding a certificate block whose content is not a certificate.
    let invalid =
        SystemStore::holding("-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n");
    // A store that cannot be read is misconfigured, not empty.
    let missing = SystemStore::holding("");
    std::fs::remove_file(missing.path()).expect("the store is deleted");
    for (case, store) in [("invalid", invalid), ("missing", missing)] {
        let output = store
            .command()
            .args(["-e", "import('ext.http').client()"])
            .output()
            .expect("`frost` should run");
        assert!(
            !output.status.success(),
            "making the client fails, with the store {case}"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(
                "Function ext.http.client could not create a client: builder error: \
                 unexpected error: No CA certificates were loaded from the system"
            ),
            "the error gives its causes, with the store {case}: {stderr}"
        );
    }
}
