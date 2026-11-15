//! Certificates for the HTTPS test server: certificate authorities of the
//! tests' own, which no system trusts, and server certificates they sign.

use std::sync::Arc;

use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair,
};
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};

/// `pem` as a Frost String literal, its line breaks escaped.
pub(crate) fn pem_literal(pem: &str) -> String {
    format!("'{}'", pem.trim_end().replace('\n', "\\n"))
}

/// A certificate authority made for a test.
pub(crate) struct TestCa {
    issuer: CertifiedIssuer<'static, KeyPair>,
}

impl TestCa {
    /// A new authority, with `name` as its common name.
    pub(crate) fn new(name: &str) -> Self {
        let mut params = CertificateParams::new(Vec::<String>::new()).expect("no names are valid");
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.distinguished_name.push(DnType::CommonName, name);
        let key = KeyPair::generate().expect("a key can be made");
        let issuer = CertifiedIssuer::self_signed(params, key).expect("an authority signs itself");
        Self { issuer }
    }

    /// The authority's certificate, as PEM content.
    pub(crate) fn pem(&self) -> String {
        self.issuer.pem()
    }

    /// A server's TLS settings, presenting a certificate this authority
    /// signed for `names`: each a DNS name or an IP address.
    pub(crate) fn server_config(&self, names: &[&str]) -> Arc<rustls::ServerConfig> {
        let names: Vec<String> = names.iter().map(ToString::to_string).collect();
        let mut params = CertificateParams::new(names).expect("the names are valid");
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
