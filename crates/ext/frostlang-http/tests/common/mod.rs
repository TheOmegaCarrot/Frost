//! Harness for the extension's tests: scripts run with only `ext.http`
//! installed, bound as `http`, against a [`Server`] of the test's own.

pub(crate) mod server;
pub(crate) mod tls;

use std::fmt::Display;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock, mpsc};
use std::thread;
use std::time::Duration;

use flate2::Compression;
use flate2::write::GzEncoder;
use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{
    AbortReason, CancelToken, Importer, ImporterBuilder, MapKey, RunErrorKind, Value, Vm,
};
use frostlang_http::HttpConfig;
use frostlang_tokio::Runtime;
use socket2::{Domain, Protocol, Socket, Type};

pub(crate) use server::Server;
pub(crate) use tls::TestCa;

/// How long a script may take before the test fails.
/// Generous: it only bounds a hang, and a passing script takes milliseconds.
pub(crate) const DEADLINE: Duration = Duration::from_secs(30);

/// `text`, gzipped, as a server compresses a body.
pub(crate) fn gzip(text: &str) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(text.as_bytes())
        .expect("writing to memory succeeds");
    encoder.finish().expect("writing to memory succeeds")
}

/// `text`, compressed with Brotli, as a server compresses a body.
pub(crate) fn brotli(text: &str) -> Vec<u8> {
    let mut encoder = brotli::CompressorWriter::new(Vec::new(), 4096, 5, 22);
    encoder
        .write_all(text.as_bytes())
        .expect("writing to memory succeeds");
    encoder.into_inner()
}

/// `text`, compressed with Zstandard, as a server compresses a body.
pub(crate) fn zstd(text: &str) -> Vec<u8> {
    zstd::encode_all(text.as_bytes(), 0).expect("compressing in memory succeeds")
}

/// A port held bound but not listening: connecting to it is refused, and no
/// other test's server can take it while this lives.
pub(crate) struct ClosedPort {
    _socket: Socket,
    address: SocketAddr,
}

impl ClosedPort {
    pub(crate) fn new() -> Self {
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))
            .expect("a socket can be made");
        let any_port: SocketAddr = "127.0.0.1:0".parse().expect("a valid address");
        socket.bind(&any_port.into()).expect("a port is free");
        let address = socket
            .local_addr()
            .ok()
            .and_then(|address| address.as_socket())
            .expect("a bound socket has an address");
        Self {
            _socket: socket,
            address,
        }
    }

    /// An http URL on this port.
    pub(crate) fn url(&self) -> String {
        format!("http://{}/", self.address)
    }
}

/// The runtime every test's requests run on, as one host's would.
pub(crate) fn runtime() -> Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(Runtime::new).clone()
}

/// An importer providing only the extension, under `config`.
pub(crate) fn importer(config: HttpConfig) -> Arc<Importer> {
    ImporterBuilder::new()
        .with_extension(frostlang_http::extension(config))
        .expect("a lone extension is accepted")
        .build()
}

/// How a run ended: its value, or how it failed.
#[derive(Debug, PartialEq)]
pub(crate) enum Ended {
    Finished(Value),
    Raised(String),
    Aborted(AbortReason),
}

/// Runs `body`, Frost source with the extension bound as `http`, with
/// `importer` and cancellable by `token`, on a thread of its own, failing the
/// test if it does not end within [`DEADLINE`].
pub(crate) fn run_with(body: &str, importer: Arc<Importer>, token: Option<&CancelToken>) -> Ended {
    check_no_ambient_proxy();
    let source = format!("def http = import('ext.http')\n{body}");
    let closure = compile_program("main.frst", &source, CompilerOptions::new())
        .unwrap_or_else(|errors| panic!("the script compiles:\n{}", errors.render()))
        .code
        .into_closure()
        .expect("the script captures nothing");
    let mut vm = Vm::factory().with_importer(importer).build(closure);
    if let Some(token) = token {
        vm = vm.with_cancel_token(token.clone());
    }
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let ended = match vm.run() {
            Ok(result) => Ended::Finished(result.tail().clone()),
            Err(failure) => match failure.kind() {
                RunErrorKind::Aborted(reason) => Ended::Aborted(reason),
                RunErrorKind::Raised => Ended::Raised(failure.into_error().message().into_owned()),
            },
        };
        sender.send(ended).expect("the test is waiting");
    });
    match receiver.recv_timeout(DEADLINE) {
        Ok(ended) => ended,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            panic!("the script did not end within {DEADLINE:?}:\n{body}")
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("the script panicked:\n{body}"),
    }
}

/// Fails the test if the environment names a proxy that would take the
/// tests' requests to their own servers: a client uses the environment's
/// proxies for loopback addresses too, unless `no_proxy` exempts them. The
/// tests cannot change the environment, which every test reads.
fn check_no_ambient_proxy() {
    static PROBLEM: OnceLock<Option<String>> = OnceLock::new();
    let problem = PROBLEM.get_or_init(|| {
        // Which spelling of the variable `name` is set, if either is.
        let set_spelling = |name: &str| {
            [name.to_string(), name.to_uppercase()]
                .iter()
                .find(|name| std::env::var_os(name).is_some_and(|value| !value.is_empty()))
                .cloned()
        };
        let proxy = ["http_proxy", "https_proxy", "all_proxy"]
            .into_iter()
            .find_map(set_spelling)?;
        let exempt = set_spelling("no_proxy")
            .and_then(|name| std::env::var(name).ok())
            .is_some_and(|hosts| {
                hosts
                    .split(',')
                    .any(|host| matches!(host.trim(), "*" | "127.0.0.1" | "127.0.0.0/8"))
            });
        (!exempt).then(|| {
            format!(
                "the environment's `{proxy}` would take the tests' requests to their own \
                 servers: unset it, or add 127.0.0.1 to `no_proxy`"
            )
        })
    });
    if let Some(problem) = problem {
        panic!("{problem}");
    }
}

/// Runs `body` as [`run_with`] does, with the extension's default settings.
pub(crate) fn run(body: &str) -> Ended {
    run_with(body, importer(HttpConfig::new(runtime())), None)
}

/// The value of `body`, which must run.
#[track_caller]
pub(crate) fn value(body: &str) -> Value {
    match run(body) {
        Ended::Finished(value) => value,
        other => panic!("the script should run, but ended with {other:?}:\n{body}"),
    }
}

/// The error message `body` raises.
#[track_caller]
pub(crate) fn raises(body: &str) -> String {
    match run(body) {
        Ended::Raised(message) => message,
        other => panic!("the script should raise, but ended with {other:?}:\n{body}"),
    }
}

/// Assert each of `cases`, a script and the message it raises, raises that
/// message.
#[track_caller]
pub(crate) fn assert_raises(cases: &[(&str, &str)]) {
    for (body, message) in cases {
        assert_eq!(raises(body), *message, "the error of:\n{body}");
    }
}

/// The value at `path`, a list of keys, within `value`'s nested Maps.
#[track_caller]
pub(crate) fn at(value: &Value, path: &[&str]) -> Value {
    path.iter().fold(value.clone(), |value, key| match &value {
        Value::Map(map) => map
            .get(&MapKey::from(*key))
            .unwrap_or_else(|| panic!("{value:?} should have key `{key}`"))
            .clone(),
        other => panic!("{other:?} should be a Map with key `{key}`"),
    })
}

/// The `kind` of the failed result `result`, checking that it failed.
#[track_caller]
pub(crate) fn failure_kind(result: &Value) -> String {
    assert_eq!(
        at(result, &["ok"]),
        Value::Bool(false),
        "the result should fail: {result:?}"
    );
    match at(result, &["error", "kind"]) {
        Value::String(kind) => kind.as_str().to_string(),
        other => panic!("a failure's kind should be a String, but is {other:?}"),
    }
}

/// The `message` of the failed result `result`, checking that it failed.
#[track_caller]
pub(crate) fn failure_message(result: &Value) -> String {
    failure_kind(result);
    match at(result, &["error", "message"]) {
        Value::String(message) => message.as_str().to_string(),
        other => panic!("a failure's message should be a String, but is {other:?}"),
    }
}

/// Assert `function`, a path within the extension such as `auth.basic`,
/// raises its arity error when called with each count in `counts`, after
/// `setup`, which binds anything `function` needs. `expects` is how the error
/// states its arity, and `name` the function's name in errors.
#[track_caller]
pub(crate) fn assert_arity(
    setup: &str,
    function: &str,
    name: &str,
    expects: impl Display,
    counts: &[usize],
) {
    let expects = expects.to_string();
    let noun = if expects == "1" {
        "argument"
    } else {
        "arguments"
    };
    for &argc in counts {
        let call = format!("{setup}\n{function}({})", vec!["null"; argc].join(", "));
        assert_eq!(
            raises(&call),
            format!("Function {name} expects {expects} {noun}, but was called with {argc}"),
            "the arity error of:\n{call}"
        );
    }
}
