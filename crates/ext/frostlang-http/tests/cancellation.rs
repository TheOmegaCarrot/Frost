//! Cancelling a run stops its requests: the run ends as cancelled at once,
//! and each request in flight is abandoned, closing its connection.

use std::sync::mpsc;
use std::thread;

use frostlang::{AbortReason, CancelToken};
use frostlang_http::HttpConfig;

use crate::common::{DEADLINE, Ended, Server, importer, run_with, runtime};

/// What a [`stalling_server`] reports about each request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    /// A request arrived, and is being held.
    Arrived,
    /// A held request's connection was closed by the client.
    Closed,
}

/// A server that holds each request after sending `head`, which may be
/// nothing, until the client closes the connection, reporting both on
/// `events`.
fn stalling_server(head: &'static [u8], events: mpsc::Sender<Event>) -> Server {
    let events = std::sync::Mutex::new(events);
    Server::start(move |_, connection| {
        let send = |event| {
            let _ = events.lock().expect("not poisoned").send(event);
        };
        connection.write(head);
        send(Event::Arrived);
        connection.wait_for_close();
        send(Event::Closed);
    })
}

/// Waits for `count` of `expected` from `events`, failing the test if one
/// does not come within [`DEADLINE`].
fn expect_events(events: &mpsc::Receiver<Event>, expected: Event, count: usize) {
    for n in 0..count {
        match events.recv_timeout(DEADLINE) {
            Ok(event) if event == expected => {}
            Ok(event) => {
                panic!("expected {count} {expected:?} events, but event {n} was {event:?}")
            }
            Err(err) => panic!("expected {count} {expected:?} events, but after {n}: {err}"),
        }
    }
}

/// Runs `call` with `url` bound to `server`'s `/`, and `client` to a client
/// that waits for a response for an hour, cancelling the run once `requests`
/// requests have arrived. Checks that the run ends as cancelled and every
/// request's connection is closed well before any timeout could end them.
fn assert_cancelling_stops(
    server: &Server,
    events: mpsc::Receiver<Event>,
    requests: usize,
    call: &str,
) {
    let token = CancelToken::new();
    let canceller = {
        let token = token.clone();
        thread::spawn(move || {
            expect_events(&events, Event::Arrived, requests);
            token.cancel();
            events
        })
    };
    let script = format!(
        "def url = '{}'\ndef client = http.client({{read_timeout: 3600000000}})\n{call}",
        server.url("/")
    );
    let ended = run_with(&script, importer(HttpConfig::new(runtime())), Some(&token));
    let events = canceller.join().expect("the canceller finishes");
    assert_eq!(ended, Ended::Aborted(AbortReason::Cancelled), "{call}");
    expect_events(&events, Event::Closed, requests);
}

#[test]
fn cancelling_stops_a_request_waiting_for_its_response() {
    let (sender, events) = mpsc::channel();
    let server = stalling_server(b"", sender);
    assert_cancelling_stops(&server, events, 1, "client.get(url)");
}

#[test]
fn cancelling_stops_a_request_reading_its_body() {
    let (sender, events) = mpsc::channel();
    let server = stalling_server(
        b"HTTP/1.1 200 OK\r\ncontent-length: 100\r\n\r\npart",
        sender,
    );
    assert_cancelling_stops(&server, events, 1, "client.get(url)");
}

#[test]
fn cancelling_stops_every_request_of_a_batch() {
    let (sender, events) = mpsc::channel();
    let server = stalling_server(b"", sender);
    assert_cancelling_stops(
        &server,
        events,
        3,
        "client.request_all([{url}, {url}, {url}, {url}, {url}], {concurrency: 3})",
    );
    assert_eq!(
        server.requests().len(),
        3,
        "requests not yet started when cancelled are never sent"
    );
}
