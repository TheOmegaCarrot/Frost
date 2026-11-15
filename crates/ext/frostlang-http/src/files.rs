//! Files a request streams its body from (`body_file`), or its response's
//! body into (`save_to`).
//!
//! A request's files are checked when it is prepared, so that most problems
//! raise, but opened only once it is sent, so that a batch holds files open
//! only for the requests in flight.

use std::fs::Metadata;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use frostlang::FrostError;
use tempfile::TempPath;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error;

/// How much of a body file is read at a time.
const CHUNK_SIZE: u64 = 64 * 1024;

/// A file a request's body is streamed from.
pub(crate) struct BodyFile {
    path: String,
}

impl BodyFile {
    /// The file at `path`, for the function `function`, which must be a
    /// regular file the process can read.
    pub(crate) fn new(function: &str, path: &str) -> Result<Self, FrostError> {
        let cannot_read =
            |reason: &dyn std::fmt::Display| error::cannot_read(function, path, reason);
        // Checked before opening, since opening some other files, such as a
        // FIFO, waits indefinitely.
        let metadata = std::fs::metadata(path).map_err(|err| cannot_read(&err))?;
        if let Some(reason) = not_regular(&metadata) {
            return Err(cannot_read(&reason));
        }
        // Opened only to check it can be, and closed at once, so that a batch
        // holds no file open until its request is sent.
        std::fs::File::open(path).map_err(|err| cannot_read(&err))?;
        Ok(Self {
            path: path.to_owned(),
        })
    }

    /// The file's path, as given.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    /// Opens the file, to send it as it is now.
    pub(crate) async fn open(&self) -> io::Result<Upload> {
        // Checked before opening, for the reason `new` gives, and again after,
        // since the file may have been replaced in between.
        let check = |metadata: &Metadata| match not_regular(metadata) {
            Some(reason) => Err(io::Error::other(reason)),
            None => Ok(()),
        };
        check(&tokio::fs::metadata(&self.path).await?)?;
        let file = tokio::fs::File::open(&self.path).await?;
        let metadata = file.metadata().await?;
        check(&metadata)?;
        let length = metadata.len();
        let error = Arc::new(OnceLock::new());
        let sending = Sending {
            file,
            remaining: length,
            error: Arc::clone(&error),
        };
        let body = reqwest::Body::wrap_stream(futures_util::stream::try_unfold(
            sending,
            Sending::next_chunk,
        ));
        Ok(Upload {
            body,
            length,
            failure: ReadFailure {
                path: self.path.clone(),
                error,
            },
        })
    }
}

/// A body file, opened to send.
pub(crate) struct Upload {
    /// Exactly `length` bytes of the file, failing if it has fewer.
    pub(crate) body: reqwest::Body,
    pub(crate) length: u64,
    pub(crate) failure: ReadFailure,
}

/// Why reading a body file failed while it was sent, if it did: recorded
/// apart from the request's own error, which, over HTTP/2, does not carry it.
pub(crate) struct ReadFailure {
    /// The file's path, as given.
    path: String,
    error: Arc<OnceLock<io::Error>>,
}

impl ReadFailure {
    /// The file's path and why reading it failed, if it did.
    pub(crate) fn get(&self) -> Option<(&str, &io::Error)> {
        Some((&self.path, self.error.get()?))
    }
}

/// A body file being sent: the open file, how much of it is still to send,
/// and where to record why reading it failed.
struct Sending {
    file: tokio::fs::File,
    remaining: u64,
    error: Arc<OnceLock<io::Error>>,
}

impl Sending {
    /// The next chunk of the body, and the state to send the rest from; none
    /// once `remaining` is sent, and a failure, also recorded, if the file
    /// cannot be read or ends first.
    async fn next_chunk(mut self) -> io::Result<Option<(Vec<u8>, Self)>> {
        if self.remaining == 0 {
            return Ok(None);
        }
        let size = usize::try_from(self.remaining.min(CHUNK_SIZE)).expect("a chunk fits in memory");
        let mut chunk = vec![0; size];
        let read = match self.file.read(&mut chunk).await {
            Ok(0) => Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("it ended {} bytes short of its length", self.remaining),
            )),
            read => read,
        };
        match read {
            Ok(read) => {
                chunk.truncate(read);
                self.remaining -= u64::try_from(read).expect("a chunk's size fits in u64");
                Ok(Some((chunk, self)))
            }
            Err(err) => {
                let reported = io::Error::new(err.kind(), err.to_string());
                let _ = self.error.set(err);
                Err(reported)
            }
        }
    }
}

/// Why a file described by `metadata` cannot be sent as a body, if it cannot:
/// only a regular file has a length to send.
fn not_regular(metadata: &Metadata) -> Option<&'static str> {
    if metadata.is_dir() {
        Some("it is a directory")
    } else if !metadata.is_file() {
        Some("it is not a regular file")
    } else {
        None
    }
}

/// Where a response's body is saved: a partial file beside the destination,
/// written as the body arrives and renamed onto the destination once it is
/// whole, so that the destination never holds part of a body.
pub(crate) struct Destination {
    /// Naming a file in an existing directory.
    absolute: PathBuf,
    /// `absolute` as text, for the result and messages.
    path: String,
}

impl Destination {
    /// A destination at `path`, for the function `function`, checked as far as
    /// it can be without creating anything.
    pub(crate) fn new(function: &str, path: &str) -> Result<Self, FrostError> {
        let cannot_save =
            |reason: &dyn std::fmt::Display| error::cannot_save(function, path, reason);
        if path.contains('\0') {
            return Err(cannot_save(&"it contains a NUL character"));
        }
        // Checked in the text, since `Path` ignores a trailing separator or
        // `.`, which would make this the name of its directory.
        if matches!(
            path.rsplit(std::path::is_separator).next(),
            Some("" | "." | "..")
        ) {
            return Err(cannot_save(&"it names no file"));
        }
        let absolute = std::path::absolute(path).map_err(|err| cannot_save(&err))?;
        let text = absolute
            .to_str()
            .ok_or_else(|| cannot_save(&"its absolute path is not UTF-8"))?
            .to_owned();
        if absolute.is_dir() {
            return Err(cannot_save(&"it is a directory"));
        }
        if !absolute.parent().is_some_and(std::path::Path::is_dir) {
            return Err(cannot_save(&"its directory does not exist"));
        }
        Ok(Self {
            absolute,
            path: text,
        })
    }

    /// The destination's absolute path, as text.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    /// Creates the partial file to write the body into.
    pub(crate) async fn create(&self) -> io::Result<Partial> {
        let destination = self.absolute.clone();
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let directory = destination.parent().expect("a destination names a file");
            let mut builder = tempfile::Builder::new();
            builder.prefix(".frost-http-").suffix(".part");
            // The permissions of any file a program creates, rather than the
            // owner-only ones a temporary file gets.
            #[cfg(unix)]
            builder.permissions(std::os::unix::fs::PermissionsExt::from_mode(0o666));
            let (file, temp) = builder.tempfile_in(directory)?.into_parts();
            Ok(Partial {
                file: tokio::fs::File::from_std(file),
                temp,
                destination,
                path,
            })
        })
        .await
        .map_err(io::Error::other)?
    }
}

/// A partial file a body is written into, beside its destination, and
/// renamed onto it once the body is whole. Dropping it first deletes the
/// partial file.
pub(crate) struct Partial {
    file: tokio::fs::File,
    temp: TempPath,
    destination: PathBuf,
    /// `destination` as text.
    path: String,
}

impl Partial {
    /// The destination's absolute path, as text.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    /// Writes `chunk`, the next part of the body.
    pub(crate) async fn write(&mut self, chunk: &[u8]) -> io::Result<()> {
        self.file.write_all(chunk).await
    }

    /// Renames the partial file onto the destination, replacing any file
    /// there.
    pub(crate) async fn finish(mut self) -> io::Result<()> {
        // Writes finish in the background until flushed.
        self.file.flush().await?;
        drop(self.file);
        tokio::task::spawn_blocking(move || {
            self.temp
                .persist(&self.destination)
                .map_err(|err| err.error)
        })
        .await
        .map_err(io::Error::other)?
    }
}
