//! The first-party extensions `frost` installs.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::process::Command;
use std::thread::JoinHandle;

/// What `frost -e code` prints, which must succeed.
fn eval(code: &str) -> String {
    eval_with(code, Command::new(env!("CARGO_BIN_EXE_frost")))
}

/// What `frost -e code` prints with the system time zone set to `tz`, which
/// must succeed. Only the child's environment changes.
fn eval_in_zone(code: &str, tz: &str) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_frost"));
    command.env("TZ", tz);
    eval_with(code, command)
}

/// What `command`, running `frost`, prints for `-e code`, which must succeed.
pub(crate) fn eval_with(code: &str, mut command: Command) -> String {
    let output = command
        .args(["-e", code])
        .output()
        .expect("`frost` should run");
    assert!(
        output.status.success(),
        "{code:?} should succeed, but failed with: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("the output is UTF-8")
}

#[test]
fn uuid_is_installed() {
    let code = r"
        def uuid = import('ext.uuid')
        print(uuid.version(uuid.v4()))
    ";
    assert_eq!(eval(code), "4\n");
}

/// A server on the loopback interface answering one request with `body`, and
/// its thread, which gives the head of the request it answered.
pub(crate) fn answer_once(body: &'static str) -> (SocketAddr, JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port is free");
    let address = listener
        .local_addr()
        .expect("a bound socket has an address");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("the script connects");
        let mut head = Vec::new();
        let mut byte = [0];
        while !head.ends_with(b"\r\n\r\n") {
            stream
                .read_exact(&mut byte)
                .expect("the request arrives whole");
            head.push(byte[0]);
        }
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("the script reads the response");
        String::from_utf8(head).expect("the request head is UTF-8")
    });
    (address, server)
}

#[test]
fn http_is_installed() {
    let (address, server) = answer_once("hello");
    let code = format!(
        r"
        def http = import('ext.http')
        print(http.client().get('http://{address}/').response.body)
        "
    );
    assert_eq!(eval_with(&code, without_proxies()), "hello\n");
    server.join().expect("the server answered");
}

/// A `frost` command with no proxy settings in its environment.
pub(crate) fn without_proxies() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_frost"));
    for name in ["http_proxy", "https_proxy", "all_proxy", "no_proxy"] {
        command.env_remove(name).env_remove(name.to_uppercase());
    }
    command
}

/// A `frost` command whose only proxy setting is `http_proxy`, naming
/// `proxy`.
fn with_http_proxy(proxy: SocketAddr) -> Command {
    let mut command = without_proxies();
    command.env("http_proxy", format!("http://{proxy}"));
    command
}

#[test]
fn http_uses_the_environments_proxy_by_default() {
    let (proxy, server) = answer_once("hello");
    let code = r"
        def http = import('ext.http')
        print(http.client().get('http://far.invalid/page').response.body)
    ";
    assert_eq!(eval_with(code, with_http_proxy(proxy)), "hello\n");
    let head = server.join().expect("the proxy answered");
    assert!(
        head.starts_with("GET http://far.invalid/page HTTP/1.1\r\n"),
        "the request goes through the proxy: {head}"
    );
}

#[test]
fn http_with_proxy_false_ignores_the_environments() {
    // The environment's proxy applies to loopback addresses too, so only
    // `proxy: false` sends this request to the server directly.
    // Each answers differently, so the output tells which got the request.
    let (proxy, _proxy_server) = answer_once("proxy");
    let (address, server) = answer_once("direct");
    let code = format!(
        r"
        def http = import('ext.http')
        print(http.client({{proxy: false}}).get('http://{address}/page').response.body)
        "
    );
    assert_eq!(eval_with(&code, with_http_proxy(proxy)), "direct\n");
    let head = server.join().expect("the server answered");
    assert!(
        head.starts_with("GET /page HTTP/1.1\r\n"),
        "the request goes to the server directly: {head}"
    );
}

#[test]
fn http_by_default_sends_even_a_loopback_request_through_the_environments_proxy() {
    let (proxy, proxy_server) = answer_once("proxy");
    let (address, _server) = answer_once("direct");
    let code = format!(
        r"
        def http = import('ext.http')
        print(http.client().get('http://{address}/page').response.body)
        "
    );
    assert_eq!(eval_with(&code, with_http_proxy(proxy)), "proxy\n");
    let head = proxy_server.join().expect("the proxy answered");
    assert!(
        head.starts_with(&format!("GET http://{address}/page HTTP/1.1\r\n")),
        "the request goes through the proxy: {head}"
    );
}

#[test]
fn http_saves_a_relative_path_under_the_working_directory() {
    let (address, server) = answer_once("hello");
    let dir = tempfile::tempdir().expect("a scratch directory can be made");
    let mut command = without_proxies();
    command.current_dir(dir.path());
    let code = format!(
        r"
        def http = import('ext.http')
        print(http.client().get('http://{address}/', {{save_to: 'out.txt'}}).response.file)
        "
    );
    let printed = eval_with(&code, command);
    server.join().expect("the server answered");
    let expected = std::fs::canonicalize(dir.path())
        .expect("the directory exists")
        .join("out.txt");
    assert_eq!(
        printed.trim_end(),
        expected.to_str().expect("UTF-8"),
        "the file is reported by its absolute path"
    );
    assert_eq!(
        std::fs::read_to_string(&expected).expect("the file was saved"),
        "hello"
    );
}

#[test]
fn datetime_is_installed() {
    let code = r"
        def dt = import('ext.datetime')
        print(dt.instant('1970-01-01T00:00:01Z'))
    ";
    assert_eq!(eval(code), "1000000\n");
}

#[test]
fn datetime_uses_the_system_time_zone_by_default() {
    // 2024-03-16T14:30:00Z is 10:30 in New York.
    let code = r"
        def dt = import('ext.datetime')
        print(dt.local_zone())
        print(dt.to_rfc9557(1710599400000000))
        print(dt.instant({year: 2024, month: 3, day: 16, hour: 10, minute: 30}))
        print(dt.format(1710599400000000, '%H:%M %z'))
        print(dt.weekday(1710644400000000))
    ";
    assert_eq!(
        eval_in_zone(code, "America/New_York"),
        "America/New_York\n2024-03-16T10:30:00-04:00[America/New_York]\n1710599400000000\n\
         10:30 -0400\n6\n",
        "each default is New York's; 2024-03-17T03:00Z, a Sunday in UTC, is Saturday there"
    );
}

#[test]
fn datetime_in_a_system_zone_without_a_name_gives_offsets() {
    let code = r"
        def dt = import('ext.datetime')
        print(dt.local_zone())
        print(dt.to_rfc9557(1710599400000000))
        def components = dt.components(1710599400000000)
        print([has(components, 'offset'), has(components, 'zone')])
    ";
    assert_eq!(
        eval_in_zone(code, "EST5EDT,M3.2.0,M11.1.0"),
        "null\n2024-03-16T10:30:00-04:00\n[ true, false ]\n",
        "a POSIX rule names no zone, so components have an offset and no zone"
    );
}

#[test]
fn formats_is_installed_with_every_format() {
    let code = r"
        print(sorted(keys(import('ext.formats'))) == ['cbor', 'json', 'msgpack', 'toml', 'u64', 'yaml'])
    ";
    assert_eq!(eval(code), "true\n");
}

#[test]
fn formats_json_round_trips() {
    let code = r#"
        def json = import('ext.formats.json')
        print(json.encode(json.decode('{"a": [1]}')) == '{"a":[1]}')
    "#;
    assert_eq!(eval(code), "true\n");
}

#[test]
fn formats_toml_round_trips() {
    let code = r"
        def toml = import('ext.formats.toml')
        def text = 'when = 1979-05-27\n\n[t]\na = [1, 2]\n'
        print(toml.encode(toml.decode(text)) == text)
    ";
    assert_eq!(eval(code), "true\n");
}

#[test]
fn formats_msgpack_round_trips() {
    let code = r"
        def msgpack = import('ext.formats.msgpack')
        # {a: x'00', b: the Uint64 18446744073709551615}
        def data = x'82a161c40100a162cfffffffffffffffff'
        print(msgpack.encode(msgpack.decode(data)) == data)
    ";
    assert_eq!(eval(code), "true\n");
}

#[test]
fn formats_cbor_round_trips() {
    let code = r"
        def cbor = import('ext.formats.cbor')
        # {a: x'00', b: the Uint64 18446744073709551615}
        def data = x'a26161410061621bffffffffffffffff'
        print(cbor.encode(cbor.decode(data)) == data)
    ";
    assert_eq!(eval(code), "true\n");
}

#[test]
fn formats_yaml_round_trips() {
    let code = r#"
        def yaml = import('ext.formats.yaml')
        def text = 'a:\n- 1\n- "true"\nb: 1.0\n'
        print(yaml.encode(yaml.decode(text)) == text)
    "#;
    assert_eq!(eval(code), "true\n");
}

/// `frost` links the bundled SQLite, whose header the bindings were built
/// from.
#[test]
fn sqlite_is_installed_bundled() {
    let code = r"
        def sqlite = import('ext.sqlite')
        print(sqlite.version)
    ";
    let bundled = frostlang_sqlite::rusqlite::ffi::SQLITE_VERSION
        .to_str()
        .expect("the version is UTF-8");
    assert_eq!(eval(code), format!("{bundled}\n"));
}
