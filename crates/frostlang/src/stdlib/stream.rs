//! Readers and writers: the closure bundles through which scripts use streams,
//! shared by the standard streams in `os`, file streams in `fs`, and in-memory
//! buffers in `string`.
//!
//! A bundle holds only the operations its stream supports, so a key it lacks
//! is an operation the stream cannot do. Text reads return Strings and raise on
//! content that is not UTF-8; binary reads return Bytes. Positions count bytes.

use std::fs::File;
use std::io::{
    self, BufRead, BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Stderr, Stdin, Stdout, Write,
};
use std::sync::{Arc, Mutex};

use crate::{Arity, FrostBytes, FrostError, FrostResult, FrostType, MapKey, Param, Params, Value};

/// Which operations a bundle offers, beyond those every reader or writer has.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// A stream that only flows: no position, and never closed by a script.
    Stream,
    /// A file: positioned, and closable.
    File,
    /// A file opened to append: closable, but with no position, since every
    /// write goes to its end.
    AppendFile,
    /// An in-memory buffer: positioned, and a writer's content can be read back.
    Buffer,
}

impl Kind {
    fn seeks(self) -> bool {
        matches!(self, Kind::File | Kind::Buffer)
    }

    fn closes(self) -> bool {
        matches!(self, Kind::File | Kind::AppendFile)
    }
}

/// The error for `function` failing with `err`.
fn io_error(function: &str, err: &io::Error) -> FrostError {
    FrostError::from_string(format!("Function {function} failed: {err}"))
}

/// The error for `function` used on a closed stream.
fn closed_error(function: &str) -> FrostError {
    FrostError::from_string(format!(
        "Function {function} requires an open stream, but it is closed"
    ))
}

/// `bytes`, which `function` read, as text.
fn text(function: &str, bytes: Vec<u8>) -> FrostResult {
    String::from_utf8(bytes).map(Value::from).map_err(|_| {
        FrostError::from_string(format!("Function {function} read text that is not UTF-8"))
    })
}

/// A type-checked Int argument as a count or position of at least 0.
fn count_arg(function: &str, position: usize, arg: &Value) -> Result<u64, FrostError> {
    let n = arg.as_int().expect("type-checked as an Int");
    u64::try_from(n).map_err(|_| {
        FrostError::from_string(format!(
            "Function {function} requires argument {position} to be at least 0, got {n}"
        ))
    })
}

const ONE_COUNT: Params = Params::new(&[Param::of(FrostType::INT)]);

/// A bundle built from `entries`.
fn bundle(entries: Vec<(&str, Value)>) -> Value {
    entries
        .into_iter()
        .map(|(key, function)| (MapKey::from(key), function))
        .collect()
}

// --- Readers ---

/// What a reader reads from.
pub(super) trait Source: BufRead + Send + 'static {
    /// This source as seekable, if it is.
    fn seekable(&mut self) -> Option<&mut dyn Seek> {
        None
    }
}

impl Source for BufReader<Stdin> {}

impl Source for BufReader<File> {
    fn seekable(&mut self) -> Option<&mut dyn Seek> {
        Some(self)
    }
}

impl Source for Cursor<FrostBytes> {
    fn seekable(&mut self) -> Option<&mut dyn Seek> {
        Some(self)
    }
}

/// A reader's source, shared by its functions; `None` once closed.
type SharedSource = Arc<Mutex<Option<Box<dyn Source>>>>;

/// Runs `operation` on the reader's source, as the function `name`.
fn with_source<T>(
    source: &SharedSource,
    name: &str,
    operation: impl FnOnce(&mut dyn Source) -> io::Result<T>,
) -> Result<T, FrostError> {
    // No operation panics while holding the lock, so it is never poisoned.
    let mut source = source.lock().expect("a stream operation never panics");
    let source = source.as_deref_mut().ok_or_else(|| closed_error(name))?;
    operation(source).map_err(|err| io_error(name, &err))
}

/// A reader bundle over `source`, offering what `kind` allows.
pub(super) fn reader(source: impl Source, kind: Kind) -> Value {
    let source: SharedSource = Arc::new(Mutex::new(Some(Box::new(source))));
    let mut entries = vec![
        ("read_line", read_line(&source)),
        ("read_one", read_one(&source)),
        ("read_rest", read_rest(&source, "reader.read_rest", true)),
        ("read_bytes", read_bytes(&source)),
        (
            "read_rest_bytes",
            read_rest(&source, "reader.read_rest_bytes", false),
        ),
        ("eof", eof(&source)),
    ];
    if kind.seeks() {
        entries.push(("tell", reader_tell(&source)));
        entries.push(("seek", reader_seek(&source)));
    }
    if kind.closes() {
        entries.push(("close", reader_close(&source)));
        entries.push(("is_open", reader_is_open(&source)));
    }
    bundle(entries)
}

fn read_line(source: &SharedSource) -> Value {
    let source = Arc::clone(source);
    Value::native("reader.read_line", Arity::Exact(0), move |_, _| {
        let line = with_source(&source, "reader.read_line", |source| {
            let mut line = Vec::new();
            if source.read_until(b'\n', &mut line)? == 0 {
                return Ok(None);
            }
            // The line ending, `\n` or `\r\n`, is not part of the line.
            if line.ends_with(b"\n") {
                line.pop();
                if line.ends_with(b"\r") {
                    line.pop();
                }
            }
            Ok(Some(line))
        })?;
        line.map_or(Ok(Value::Null), |line| text("reader.read_line", line))
    })
}

fn read_one(source: &SharedSource) -> Value {
    let source = Arc::clone(source);
    Value::native("reader.read_one", Arity::Exact(0), move |_, _| {
        let encoded = with_source(&source, "reader.read_one", |source| {
            let Some(&first) = source.fill_buf()?.first() else {
                return Ok(None);
            };
            // A UTF-8 character's first byte says how many bytes it has. A byte
            // that cannot start one is read alone, and fails as text below.
            let width = match first {
                0xc0..=0xdf => 2,
                0xe0..=0xef => 3,
                0xf0..=0xf7 => 4,
                _ => 1,
            };
            let mut encoded = Vec::with_capacity(width);
            source.take(width as u64).read_to_end(&mut encoded)?;
            Ok(Some(encoded))
        })?;
        encoded.map_or(Ok(Value::Null), |encoded| text("reader.read_one", encoded))
    })
}

/// `read_rest`, as text, or `read_rest_bytes`, as Bytes.
fn read_rest(source: &SharedSource, name: &'static str, as_text: bool) -> Value {
    let source = Arc::clone(source);
    Value::native(name, Arity::Exact(0), move |_, _| {
        let rest = with_source(&source, name, |source| {
            let mut rest = Vec::new();
            source.read_to_end(&mut rest)?;
            Ok(rest)
        })?;
        if as_text {
            text(name, rest)
        } else {
            Ok(Value::from(rest))
        }
    })
}

fn read_bytes(source: &SharedSource) -> Value {
    let source = Arc::clone(source);
    Value::checked_native("reader.read_bytes", ONE_COUNT, move |_, args| {
        let n = count_arg("reader.read_bytes", 1, &args[0])?;
        let bytes = with_source(&source, "reader.read_bytes", |source| {
            let mut bytes = Vec::new();
            source.take(n).read_to_end(&mut bytes)?;
            Ok(bytes)
        })?;
        // Nothing left to read is Null; a count of 0 reads nothing, at no end.
        Ok(if bytes.is_empty() && n > 0 {
            Value::Null
        } else {
            Value::from(bytes)
        })
    })
}

fn eof(source: &SharedSource) -> Value {
    let source = Arc::clone(source);
    Value::native("reader.eof", Arity::Exact(0), move |_, _| {
        let at_end = with_source(&source, "reader.eof", |source| {
            Ok(source.fill_buf()?.is_empty())
        })?;
        Ok(Value::Bool(at_end))
    })
}

fn reader_tell(source: &SharedSource) -> Value {
    let source = Arc::clone(source);
    Value::native("reader.tell", Arity::Exact(0), move |_, _| {
        let position = with_source(&source, "reader.tell", |source| {
            seekable(source.seekable()).stream_position()
        })?;
        Ok(position_value(position))
    })
}

fn reader_seek(source: &SharedSource) -> Value {
    let source = Arc::clone(source);
    Value::checked_native("reader.seek", ONE_COUNT, move |_, args| {
        let position = count_arg("reader.seek", 1, &args[0])?;
        with_source(&source, "reader.seek", |source| {
            seekable(source.seekable()).seek(SeekFrom::Start(position))
        })?;
        Ok(Value::Null)
    })
}

fn reader_close(source: &SharedSource) -> Value {
    let source = Arc::clone(source);
    Value::native("reader.close", Arity::Exact(0), move |_, _| {
        // Closing a closed stream does nothing.
        drop(
            source
                .lock()
                .expect("a stream operation never panics")
                .take(),
        );
        Ok(Value::Null)
    })
}

fn reader_is_open(source: &SharedSource) -> Value {
    let source = Arc::clone(source);
    Value::native("reader.is_open", Arity::Exact(0), move |_, _| {
        let open = source
            .lock()
            .expect("a stream operation never panics")
            .is_some();
        Ok(Value::Bool(open))
    })
}

// --- Writers ---

/// What a writer writes to.
pub(super) trait Sink: Write + Send + 'static {
    /// This sink as seekable, if it is.
    fn seekable(&mut self) -> Option<&mut dyn Seek> {
        None
    }

    /// Everything written to this sink, if it keeps it.
    fn contents(&self) -> Option<&[u8]> {
        None
    }
}

impl Sink for Stdout {}

impl Sink for Stderr {}

impl Sink for BufWriter<File> {
    fn seekable(&mut self) -> Option<&mut dyn Seek> {
        Some(self)
    }
}

/// An in-memory sink, which keeps everything written to it.
#[derive(Default)]
pub(super) struct Buffer(Cursor<Vec<u8>>);

impl Write for Buffer {
    fn write(&mut self, content: &[u8]) -> io::Result<usize> {
        // Writing grows the buffer to the write's end, which may be far past
        // the content after a seek. Past `isize::MAX` bytes, that would panic.
        let fits = usize::try_from(self.0.position())
            .ok()
            .and_then(|position| position.checked_add(content.len()))
            .is_some_and(|end| isize::try_from(end).is_ok());
        if !fits {
            return Err(io::Error::new(
                io::ErrorKind::OutOfMemory,
                "the buffer cannot grow that large",
            ));
        }
        self.0.write(content)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for Buffer {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.0.seek(position)
    }
}

impl Sink for Buffer {
    fn seekable(&mut self) -> Option<&mut dyn Seek> {
        Some(self)
    }

    fn contents(&self) -> Option<&[u8]> {
        Some(self.0.get_ref())
    }
}

/// A writer's sink, shared by its functions; `None` once closed.
type SharedSink = Arc<Mutex<Option<Box<dyn Sink>>>>;

/// Runs `operation` on the writer's sink, as the function `name`.
fn with_sink<T>(
    sink: &SharedSink,
    name: &str,
    operation: impl FnOnce(&mut dyn Sink) -> io::Result<T>,
) -> Result<T, FrostError> {
    // No operation panics while holding the lock, so it is never poisoned.
    let mut sink = sink.lock().expect("a stream operation never panics");
    let sink = sink.as_deref_mut().ok_or_else(|| closed_error(name))?;
    operation(sink).map_err(|err| io_error(name, &err))
}

/// A writer bundle over `sink`, offering what `kind` allows.
pub(super) fn writer(sink: impl Sink, kind: Kind) -> Value {
    let sink: SharedSink = Arc::new(Mutex::new(Some(Box::new(sink))));
    let mut entries = vec![
        ("write", write(&sink, "writer.write", false)),
        ("writeln", write(&sink, "writer.writeln", true)),
    ];
    if kind != Kind::Buffer {
        entries.push(("flush", flush(&sink)));
    }
    if kind.seeks() {
        entries.push(("tell", writer_tell(&sink)));
        entries.push(("seek", writer_seek(&sink)));
    }
    if kind.closes() {
        entries.push(("close", writer_close(&sink)));
        entries.push(("is_open", writer_is_open(&sink)));
    }
    if kind == Kind::Buffer {
        entries.push(("get", get(&sink, "writer.get", true)));
        entries.push(("get_bytes", get(&sink, "writer.get_bytes", false)));
    }
    bundle(entries)
}

/// `write`, or `writeln` when `line`, which ends what it writes with `\n`.
fn write(sink: &SharedSink, name: &'static str, line: bool) -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::FLAT)]);
    let sink = Arc::clone(sink);
    Value::checked_native(name, PARAMS, move |_, args| {
        let content = args[0].as_byte_slice().expect("type-checked as Flat");
        with_sink(&sink, name, |sink| {
            sink.write_all(content)?;
            if line {
                sink.write_all(b"\n")?;
            }
            Ok(())
        })?;
        Ok(Value::Null)
    })
}

fn flush(sink: &SharedSink) -> Value {
    let sink = Arc::clone(sink);
    Value::native("writer.flush", Arity::Exact(0), move |_, _| {
        with_sink(&sink, "writer.flush", Write::flush)?;
        Ok(Value::Null)
    })
}

fn writer_tell(sink: &SharedSink) -> Value {
    let sink = Arc::clone(sink);
    Value::native("writer.tell", Arity::Exact(0), move |_, _| {
        let position = with_sink(&sink, "writer.tell", |sink| {
            seekable(sink.seekable()).stream_position()
        })?;
        Ok(position_value(position))
    })
}

fn writer_seek(sink: &SharedSink) -> Value {
    let sink = Arc::clone(sink);
    Value::checked_native("writer.seek", ONE_COUNT, move |_, args| {
        let position = count_arg("writer.seek", 1, &args[0])?;
        with_sink(&sink, "writer.seek", |sink| {
            seekable(sink.seekable()).seek(SeekFrom::Start(position))
        })?;
        Ok(Value::Null)
    })
}

fn writer_close(sink: &SharedSink) -> Value {
    let sink = Arc::clone(sink);
    Value::native("writer.close", Arity::Exact(0), move |_, _| {
        let mut sink = sink.lock().expect("a stream operation never panics");
        // Flushed before it is dropped, so a failure to write out is reported;
        // closing a closed stream does nothing.
        if let Some(mut open) = sink.take() {
            open.flush().map_err(|err| io_error("writer.close", &err))?;
        }
        Ok(Value::Null)
    })
}

fn writer_is_open(sink: &SharedSink) -> Value {
    let sink = Arc::clone(sink);
    Value::native("writer.is_open", Arity::Exact(0), move |_, _| {
        let open = sink
            .lock()
            .expect("a stream operation never panics")
            .is_some();
        Ok(Value::Bool(open))
    })
}

/// `get`, as text, or `get_bytes`, as Bytes: everything written so far.
fn get(sink: &SharedSink, name: &'static str, as_text: bool) -> Value {
    let sink = Arc::clone(sink);
    Value::native(name, Arity::Exact(0), move |_, _| {
        let contents = with_sink(&sink, name, |sink| {
            Ok(sink
                .contents()
                .expect("a buffer writer keeps its contents")
                .to_vec())
        })?;
        if as_text {
            text(name, contents)
        } else {
            Ok(Value::from(contents))
        }
    })
}

// --- Shared ---

/// A stream offering `tell` and `seek` can seek.
fn seekable(seekable: Option<&mut dyn Seek>) -> &mut dyn Seek {
    seekable.expect("a stream offering tell and seek can seek")
}

/// A byte position as an Int.
fn position_value(position: u64) -> Value {
    Value::Int(i64::try_from(position).expect("a stream position fits in an Int"))
}
