//! Compression is the client's business, never the script's: the client asks
//! for every compression it decompresses, and a script sees a compressed
//! response as the server's uncompressed body. A client with
//! `decompress: false` asks for none, and gives any compressed body as sent.

use frostlang::Value;

use crate::common::{Server, assert_raises, at, brotli, gzip, value, zstd};

/// A compression's `content-encoding` name, and a function compressing with
/// it.
type Coding = (&'static str, fn(&str) -> Vec<u8>);

/// Every compression the client decompresses.
const CODINGS: [Coding; 3] = [("gzip", gzip), ("br", brotli), ("zstd", zstd)];

/// The text every server here compresses: long enough to compress.
fn text() -> String {
    "Frost is an embeddable scripting language. ".repeat(20)
}

#[test]
fn every_compression_is_asked_for() {
    let server = Server::ok("");
    value(&format!("http.client().get('{}')", server.url("/")));
    let asked: Vec<String> = server
        .request()
        .header("accept-encoding")
        .iter()
        .flat_map(|value| value.split(',').map(|coding| coding.trim().to_string()))
        .collect();
    for coding in ["gzip", "br", "zstd"] {
        assert!(
            asked.iter().any(|asked| asked == coding),
            "the client asks for {coding}, among {asked:?}"
        );
    }
}

#[test]
fn a_compressed_response_reads_as_its_uncompressed_body() {
    for (coding, compress) in CODINGS {
        let server = Server::start(move |_, connection| {
            connection.respond(
                200,
                &[("content-encoding", coding), ("x-kept", "yes")],
                &compress(&text()),
            );
        });
        let result = value(&format!("http.client().get('{}')", server.url("/")));
        assert_eq!(
            at(&result, &["response", "body"]),
            Value::from(text()),
            "the body, {coding}-compressed"
        );
        // The headers describe the body as the script sees it: neither the
        // compression nor the compressed length.
        assert_eq!(
            at(&result, &["response", "headers"]),
            value("{['x-kept']: 'yes'}"),
            "the headers of a {coding}-compressed body"
        );
    }
}

#[test]
fn without_decompression_no_compression_is_asked_for() {
    let server = Server::ok("");
    value(&format!(
        "http.client({{decompress: false}}).get('{}')",
        server.url("/")
    ));
    assert_eq!(
        server.request().header("accept-encoding"),
        Vec::<String>::new(),
        "the client asks for no compression"
    );
}

#[test]
fn without_decompression_a_compressed_body_is_given_as_sent() {
    // Some servers compress what was not asked for compressed, such as a
    // `.gz` download labelled `content-encoding: gzip`.
    for (coding, compress) in CODINGS {
        let compressed = compress(&text());
        let server = Server::start(move |_, connection| {
            connection.respond(200, &[("content-encoding", coding)], &compress(&text()));
        });
        let result = value(&format!(
            "http.client({{decompress: false}}).get('{}')",
            server.url("/")
        ));
        // Compressed bytes may happen to be valid UTF-8, and so a String.
        let body = match at(&result, &["response", "body"]) {
            Value::Bytes(bytes) => bytes.to_vec(),
            Value::String(text) => text.as_str().as_bytes().to_vec(),
            other => panic!("a body is a String or Bytes, but is {other:?}"),
        };
        assert_eq!(body, compressed, "the body, {coding}-compressed");
        assert_eq!(
            at(&result, &["response", "headers"]),
            value(&format!(
                "{{['content-encoding']: '{coding}', ['content-length']: '{}'}}",
                compressed.len()
            )),
            "the headers describe the body as sent, {coding}-compressed"
        );
    }
}

#[test]
fn decompress_takes_a_bool() {
    let server = Server::start(|_, connection| {
        connection.respond(200, &[("content-encoding", "gzip")], &gzip("text"));
    });
    let result = value(&format!(
        "http.client({{decompress: true}}).get('{}')",
        server.url("/")
    ));
    assert_eq!(at(&result, &["response", "body"]), Value::from("text"));
    assert_raises(&[(
        "http.client({decompress: 'no'})",
        "Function ext.http.client requires argument 1 (config) to be valid: \
         expected Bool, got String (at `decompress`)",
    )]);
}
