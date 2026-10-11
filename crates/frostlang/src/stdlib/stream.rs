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

use crate::native::{Args, FrostArg, MutableBundle, MutableBundleCtx};
use crate::{FrostBytes, FrostError, FrostResult, FrostType, Param, Params, Value};

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

const NO_PARAMS: Params = Params::new(&[]);
const ONE_COUNT: Params = Params::new(&[<u64 as FrostArg>::PARAM]);

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

/// The context of a reader's functions, over its source; `None` once closed.
type ReaderCtx<'a> = MutableBundleCtx<'a, Option<Box<dyn Source>>>;

/// Runs `operation` on the reader's source.
fn with_source<T>(
    ctx: &mut ReaderCtx<'_>,
    operation: impl FnOnce(&mut dyn Source) -> io::Result<T>,
) -> Result<T, FrostError> {
    let mut source = ctx.lock()?;
    let name = source.native().name();
    let source = source.as_deref_mut().ok_or_else(|| closed_error(name))?;
    operation(source).map_err(|err| io_error(name, &err))
}

/// A reader bundle over `source`, offering what `kind` allows.
pub(super) fn reader(source: impl Source, kind: Kind) -> Value {
    let source: Box<dyn Source> = Box::new(source);
    let mut reader = MutableBundle::new(Some(source))
        .function("reader.read_line", NO_PARAMS, read_line)
        .function("reader.read_one", NO_PARAMS, read_one)
        .function("reader.read_rest", NO_PARAMS, |ctx, _| read_rest(ctx, true))
        .function("reader.read_bytes", ONE_COUNT, read_bytes)
        .function("reader.read_rest_bytes", NO_PARAMS, |ctx, _| {
            read_rest(ctx, false)
        })
        .function("reader.eof", NO_PARAMS, eof);
    if kind.seeks() {
        reader = reader
            .function("reader.tell", NO_PARAMS, reader_tell)
            .function("reader.seek", ONE_COUNT, reader_seek);
    }
    if kind.closes() {
        reader = reader
            .function("reader.close", NO_PARAMS, reader_close)
            .function("reader.is_open", NO_PARAMS, reader_is_open);
    }
    reader.build()
}

fn read_line(ctx: &mut ReaderCtx<'_>, _: Args<'_>) -> FrostResult {
    let line = with_source(ctx, |source| {
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
    line.map_or(Ok(Value::Null), |line| text(ctx.native().name(), line))
}

fn read_one(ctx: &mut ReaderCtx<'_>, _: Args<'_>) -> FrostResult {
    let encoded = with_source(ctx, |source| {
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
    encoded.map_or(Ok(Value::Null), |encoded| {
        text(ctx.native().name(), encoded)
    })
}

/// `read_rest`, as text, or `read_rest_bytes`, as Bytes.
fn read_rest(ctx: &mut ReaderCtx<'_>, as_text: bool) -> FrostResult {
    let rest = with_source(ctx, |source| {
        let mut rest = Vec::new();
        source.read_to_end(&mut rest)?;
        Ok(rest)
    })?;
    if as_text {
        text(ctx.native().name(), rest)
    } else {
        Ok(Value::from(rest))
    }
}

fn read_bytes(ctx: &mut ReaderCtx<'_>, mut args: Args<'_>) -> FrostResult {
    let n: u64 = args.take()?;
    let bytes = with_source(ctx, |source| {
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
}

fn eof(ctx: &mut ReaderCtx<'_>, _: Args<'_>) -> FrostResult {
    let at_end = with_source(ctx, |source| Ok(source.fill_buf()?.is_empty()))?;
    Ok(Value::Bool(at_end))
}

fn reader_tell(ctx: &mut ReaderCtx<'_>, _: Args<'_>) -> FrostResult {
    let position = with_source(ctx, |source| seekable(source.seekable()).stream_position())?;
    Ok(position_value(position))
}

fn reader_seek(ctx: &mut ReaderCtx<'_>, mut args: Args<'_>) -> FrostResult {
    let position: u64 = args.take()?;
    with_source(ctx, |source| {
        seekable(source.seekable()).seek(SeekFrom::Start(position))
    })?;
    Ok(Value::Null)
}

fn reader_close(ctx: &mut ReaderCtx<'_>, _: Args<'_>) -> FrostResult {
    // Closing a closed stream does nothing.
    drop(ctx.lock()?.take());
    Ok(Value::Null)
}

fn reader_is_open(ctx: &mut ReaderCtx<'_>, _: Args<'_>) -> FrostResult {
    Ok(Value::Bool(ctx.lock()?.is_some()))
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

/// The context of a writer's functions, over its sink; `None` once closed.
type WriterCtx<'a> = MutableBundleCtx<'a, Option<Box<dyn Sink>>>;

/// Runs `operation` on the writer's sink.
fn with_sink<T>(
    ctx: &mut WriterCtx<'_>,
    operation: impl FnOnce(&mut dyn Sink) -> io::Result<T>,
) -> Result<T, FrostError> {
    let mut sink = ctx.lock()?;
    let name = sink.native().name();
    let sink = sink.as_deref_mut().ok_or_else(|| closed_error(name))?;
    operation(sink).map_err(|err| io_error(name, &err))
}

/// A writer bundle over `sink`, offering what `kind` allows.
pub(super) fn writer(sink: impl Sink, kind: Kind) -> Value {
    const CONTENT: Params = Params::new(&[Param::of(FrostType::FLAT)]);
    let sink: Box<dyn Sink> = Box::new(sink);
    let mut writer = MutableBundle::new(Some(sink))
        .function("writer.write", CONTENT, |ctx, args| write(ctx, args, false))
        .function("writer.writeln", CONTENT, |ctx, args| {
            write(ctx, args, true)
        });
    if kind != Kind::Buffer {
        writer = writer.function("writer.flush", NO_PARAMS, flush);
    }
    if kind.seeks() {
        writer = writer
            .function("writer.tell", NO_PARAMS, writer_tell)
            .function("writer.seek", ONE_COUNT, writer_seek);
    }
    if kind.closes() {
        writer = writer
            .function("writer.close", NO_PARAMS, writer_close)
            .function("writer.is_open", NO_PARAMS, writer_is_open);
    }
    if kind == Kind::Buffer {
        writer = writer
            .function("writer.get", NO_PARAMS, |ctx, _| get(ctx, true))
            .function("writer.get_bytes", NO_PARAMS, |ctx, _| get(ctx, false));
    }
    writer.build()
}

/// `write`, or `writeln` when `line`, which ends what it writes with `\n`.
fn write(ctx: &mut WriterCtx<'_>, mut args: Args<'_>, line: bool) -> FrostResult {
    let content: Value = args.take()?;
    let content = content.as_byte_slice().expect("type-checked as Flat");
    with_sink(ctx, |sink| {
        sink.write_all(content)?;
        if line {
            sink.write_all(b"\n")?;
        }
        Ok(())
    })?;
    Ok(Value::Null)
}

fn flush(ctx: &mut WriterCtx<'_>, _: Args<'_>) -> FrostResult {
    with_sink(ctx, Write::flush)?;
    Ok(Value::Null)
}

fn writer_tell(ctx: &mut WriterCtx<'_>, _: Args<'_>) -> FrostResult {
    let position = with_sink(ctx, |sink| seekable(sink.seekable()).stream_position())?;
    Ok(position_value(position))
}

fn writer_seek(ctx: &mut WriterCtx<'_>, mut args: Args<'_>) -> FrostResult {
    let position: u64 = args.take()?;
    with_sink(ctx, |sink| {
        seekable(sink.seekable()).seek(SeekFrom::Start(position))
    })?;
    Ok(Value::Null)
}

fn writer_close(ctx: &mut WriterCtx<'_>, _: Args<'_>) -> FrostResult {
    let mut sink = ctx.lock()?;
    // Flushed before it is dropped, so a failure to write out is reported;
    // closing a closed stream does nothing.
    if let Some(mut open) = sink.take() {
        open.flush()
            .map_err(|err| io_error(sink.native().name(), &err))?;
    }
    Ok(Value::Null)
}

fn writer_is_open(ctx: &mut WriterCtx<'_>, _: Args<'_>) -> FrostResult {
    Ok(Value::Bool(ctx.lock()?.is_some()))
}

/// `get`, as text, or `get_bytes`, as Bytes: everything written so far.
fn get(ctx: &mut WriterCtx<'_>, as_text: bool) -> FrostResult {
    let contents = with_sink(ctx, |sink| {
        Ok(sink
            .contents()
            .expect("a buffer writer keeps its contents")
            .to_vec())
    })?;
    if as_text {
        text(ctx.native().name(), contents)
    } else {
        Ok(Value::from(contents))
    }
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
