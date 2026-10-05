//! Where a script's printed text goes.

use std::fmt;
use std::io::Write;

/// Receives the text a script prints, one call per `print`.
///
/// Set one on [`VmRuntimeConfiguration::print_sink`](crate::VmRuntimeConfiguration::print_sink)
/// to route a script's output: to a log, a buffer, a UI, or nowhere.
/// A closure taking `&str` is a sink.
///
/// Each call receives one `print`'s text, with no line terminator added;
/// the text itself may contain newlines.
/// A sink may be shared by several Vms, including those running imported modules,
/// and so may be called from several threads.
pub trait PrintSink: Send + Sync {
    /// Receive the text of one `print`.
    fn print(&self, text: &str);
}

impl<F> PrintSink for F
where
    F: Fn(&str) + Send + Sync,
{
    fn print(&self, text: &str) {
        self(text);
    }
}

impl fmt::Debug for dyn PrintSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrintSink")
    }
}

/// The default [`PrintSink`]: writes each text to the process's standard output,
/// followed by a newline.
///
/// A failed write, such as to a closed pipe, is ignored.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdoutSink;

impl PrintSink for StdoutSink {
    fn print(&self, text: &str) {
        let _ = writeln!(std::io::stdout().lock(), "{text}");
    }
}
