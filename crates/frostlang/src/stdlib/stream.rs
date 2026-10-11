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

use crate::native::{Flat, MutableBundle};
use crate::{FrostBytes, FrostError, FrostResult, NativeCtx, Value, frost_mutable_bundle};

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

/// A reader bundle over `source`, offering what `kind` allows.
pub(super) fn reader(source: impl Source, kind: Kind) -> Value {
    let mut reader = Reader::frost_functions(MutableBundle::new(Reader(Some(Box::new(source)))));
    if kind.seeks() {
        reader = Reader::seek_functions(reader);
    }
    if kind.closes() {
        reader = Reader::close_functions(reader);
    }
    reader.build()
}

/// A reader's source; `None` once closed.
struct Reader(Option<Box<dyn Source>>);

impl Reader {
    /// Runs `operation` on the source, for the function `ctx` runs.
    fn with_source<T>(
        &mut self,
        ctx: &NativeCtx<'_>,
        operation: impl FnOnce(&mut dyn Source) -> io::Result<T>,
    ) -> Result<T, FrostError> {
        let source = self
            .0
            .as_deref_mut()
            .ok_or_else(|| closed_error(ctx.name()))?;
        operation(source).map_err(|err| io_error(ctx.name(), &err))
    }

    /// Everything left to read.
    fn rest(&mut self, ctx: &NativeCtx<'_>) -> Result<Vec<u8>, FrostError> {
        self.with_source(ctx, |source| {
            let mut rest = Vec::new();
            source.read_to_end(&mut rest)?;
            Ok(rest)
        })
    }
}

#[frost_mutable_bundle(prefix = "reader")]
impl Reader {
    fn read_line(&mut self, #[frost(ctx)] ctx: &NativeCtx<'_>) -> FrostResult {
        let line = self.with_source(ctx, |source| {
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
        line.map_or(Ok(Value::Null), |line| text(ctx.name(), line))
    }

    fn read_one(&mut self, #[frost(ctx)] ctx: &NativeCtx<'_>) -> FrostResult {
        let encoded = self.with_source(ctx, |source| {
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
        encoded.map_or(Ok(Value::Null), |encoded| text(ctx.name(), encoded))
    }

    fn read_rest(&mut self, #[frost(ctx)] ctx: &NativeCtx<'_>) -> FrostResult {
        let rest = self.rest(ctx)?;
        text(ctx.name(), rest)
    }

    fn read_bytes(
        &mut self,
        #[frost(ctx)] ctx: &NativeCtx<'_>,
        #[frost(unnamed)] n: u64,
    ) -> FrostResult {
        let bytes = self.with_source(ctx, |source| {
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

    fn read_rest_bytes(&mut self, #[frost(ctx)] ctx: &NativeCtx<'_>) -> FrostResult {
        Ok(Value::from(self.rest(ctx)?))
    }

    fn eof(&mut self, #[frost(ctx)] ctx: &NativeCtx<'_>) -> Result<bool, FrostError> {
        self.with_source(ctx, |source| Ok(source.fill_buf()?.is_empty()))
    }
}

#[frost_mutable_bundle(prefix = "reader", group = seek_functions)]
impl Reader {
    fn tell(&mut self, #[frost(ctx)] ctx: &NativeCtx<'_>) -> FrostResult {
        let position =
            self.with_source(ctx, |source| seekable(source.seekable()).stream_position())?;
        Ok(position_value(position))
    }

    fn seek(
        &mut self,
        #[frost(ctx)] ctx: &NativeCtx<'_>,
        #[frost(unnamed)] position: u64,
    ) -> Result<(), FrostError> {
        self.with_source(ctx, |source| {
            seekable(source.seekable()).seek(SeekFrom::Start(position))
        })?;
        Ok(())
    }
}

#[frost_mutable_bundle(prefix = "reader", group = close_functions)]
impl Reader {
    fn close(&mut self) {
        // Closing a closed stream does nothing.
        drop(self.0.take());
    }

    fn is_open(&self) -> bool {
        self.0.is_some()
    }
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

/// A writer bundle over `sink`, offering what `kind` allows.
pub(super) fn writer(sink: impl Sink, kind: Kind) -> Value {
    let mut writer = Writer::frost_functions(MutableBundle::new(Writer(Some(Box::new(sink)))));
    if kind != Kind::Buffer {
        writer = Writer::flush_functions(writer);
    }
    if kind.seeks() {
        writer = Writer::seek_functions(writer);
    }
    if kind.closes() {
        writer = Writer::close_functions(writer);
    }
    if kind == Kind::Buffer {
        writer = Writer::buffer_functions(writer);
    }
    writer.build()
}

/// A writer's sink; `None` once closed.
struct Writer(Option<Box<dyn Sink>>);

impl Writer {
    /// Runs `operation` on the sink, for the function `ctx` runs.
    fn with_sink<T>(
        &mut self,
        ctx: &NativeCtx<'_>,
        operation: impl FnOnce(&mut dyn Sink) -> io::Result<T>,
    ) -> Result<T, FrostError> {
        let sink = self
            .0
            .as_deref_mut()
            .ok_or_else(|| closed_error(ctx.name()))?;
        operation(sink).map_err(|err| io_error(ctx.name(), &err))
    }

    /// Writes `content`, then a `\n` when `line`.
    fn write_content(
        &mut self,
        ctx: &NativeCtx<'_>,
        content: &Flat,
        line: bool,
    ) -> Result<(), FrostError> {
        self.with_sink(ctx, |sink| {
            sink.write_all(content.as_bytes())?;
            if line {
                sink.write_all(b"\n")?;
            }
            Ok(())
        })
    }

    /// Everything written so far.
    fn contents(&mut self, ctx: &NativeCtx<'_>) -> Result<Vec<u8>, FrostError> {
        self.with_sink(ctx, |sink| {
            Ok(sink
                .contents()
                .expect("a buffer writer keeps its contents")
                .to_vec())
        })
    }
}

#[frost_mutable_bundle(prefix = "writer")]
impl Writer {
    fn write(
        &mut self,
        #[frost(ctx)] ctx: &NativeCtx<'_>,
        #[frost(unnamed)] content: Flat,
    ) -> Result<(), FrostError> {
        self.write_content(ctx, &content, false)
    }

    fn writeln(
        &mut self,
        #[frost(ctx)] ctx: &NativeCtx<'_>,
        #[frost(unnamed)] content: Flat,
    ) -> Result<(), FrostError> {
        self.write_content(ctx, &content, true)
    }
}

#[frost_mutable_bundle(prefix = "writer", group = flush_functions)]
impl Writer {
    fn flush(&mut self, #[frost(ctx)] ctx: &NativeCtx<'_>) -> Result<(), FrostError> {
        self.with_sink(ctx, Write::flush)
    }
}

#[frost_mutable_bundle(prefix = "writer", group = seek_functions)]
impl Writer {
    fn tell(&mut self, #[frost(ctx)] ctx: &NativeCtx<'_>) -> FrostResult {
        let position = self.with_sink(ctx, |sink| seekable(sink.seekable()).stream_position())?;
        Ok(position_value(position))
    }

    fn seek(
        &mut self,
        #[frost(ctx)] ctx: &NativeCtx<'_>,
        #[frost(unnamed)] position: u64,
    ) -> Result<(), FrostError> {
        self.with_sink(ctx, |sink| {
            seekable(sink.seekable()).seek(SeekFrom::Start(position))
        })?;
        Ok(())
    }
}

#[frost_mutable_bundle(prefix = "writer", group = close_functions)]
impl Writer {
    fn close(&mut self, #[frost(ctx)] ctx: &NativeCtx<'_>) -> Result<(), FrostError> {
        // Flushed before it is dropped, so a failure to write out is reported;
        // closing a closed stream does nothing.
        if let Some(mut open) = self.0.take() {
            open.flush().map_err(|err| io_error(ctx.name(), &err))?;
        }
        Ok(())
    }

    fn is_open(&self) -> bool {
        self.0.is_some()
    }
}

#[frost_mutable_bundle(prefix = "writer", group = buffer_functions)]
impl Writer {
    fn get(&mut self, #[frost(ctx)] ctx: &NativeCtx<'_>) -> FrostResult {
        let contents = self.contents(ctx)?;
        text(ctx.name(), contents)
    }

    fn get_bytes(&mut self, #[frost(ctx)] ctx: &NativeCtx<'_>) -> FrostResult {
        Ok(Value::from(self.contents(ctx)?))
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
