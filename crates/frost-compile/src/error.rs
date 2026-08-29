//! Compiler diagnostics.
//!
//! A [`CompilerError`] is one diagnostic: a message, a severity, and any number
//! of labeled spans into a source, plus optional related sub-diagnostics that
//! render as their own separate blocks (the "this is wrong because of that over
//! there" shape, where "that" may even live in a different file). Because a
//! diagnostic carries its severity, the same type expresses both hard errors
//! and warnings.
//!
//! [`CompilerErrors`] is the plural: the set returned on the error channel when
//! compilation fails.

#![allow(unused)] // These error helpers are being written before any consumers
// Silence, Clippy

#[cfg(test)]
mod demo;

use std::fmt;

use frost_parse::ast::SourceSpan;
use miette::{
    Diagnostic, GraphicalReportHandler, GraphicalTheme, LabeledSpan, NamedSource, Severity,
    SourceCode,
};

/// A single compiler diagnostic: an error, a warning, or advice.
///
/// Constructed by the compiler; the caller consumes one by rendering it, either
/// through [`Display`](fmt::Display) (a human-friendly rendering, colored when
/// the output is a terminal) or through the explicit
/// [`render_colored`](Self::render_colored) and [`render_plain`](Self::render_plain)
/// wrappers.
#[derive(Clone, Debug)]
pub struct CompilerError(Diag);

/// The data behind a [`CompilerError`], and its `miette` view.
///
/// This is the type that implements [`Diagnostic`]. Its [`Display`](fmt::Display)
/// is the bare headline: the graphical handler uses it for the block header, so
/// it must not itself invoke rendering. The pretty, whole-report rendering lives
/// on [`CompilerError`], keeping the two roles on distinct types.
#[derive(Clone, Debug)]
struct Diag {
    severity: Severity,
    message: String,
    /// Machine-readable code, e.g. `frost::compile::unbound_name`.
    code: Option<String>,
    /// A closing hint on how to fix the problem.
    help: Option<String>,
    /// The source these labels point into.
    ///
    /// A related diagnostic may leave this `None` to inherit its parent's
    /// source; set it when the diagnostic points into a different source.
    source: Option<NamedSource<String>>,
    /// Labeled spans, all resolved against `source`. A primary label (see
    /// [`CompilerError::label_primary`]) frames the snippet.
    labels: Vec<LabeledSpan>,
    /// Sub-diagnostics, each rendered as its own block.
    related: Vec<Diag>,
}

// -- Construction (crate-internal) --

impl CompilerError {
    /// A diagnostic at the given severity.
    pub(crate) fn new(severity: Severity, message: String) -> Self {
        Self(Diag {
            severity,
            message,
            code: None,
            help: None,
            source: None,
            labels: Vec::new(),
            related: Vec::new(),
        })
    }

    /// A hard error: compilation cannot succeed.
    pub(crate) fn error(message: String) -> Self {
        Self::new(Severity::Error, message)
    }

    /// A warning: compilation can still succeed.
    pub(crate) fn warning(message: String) -> Self {
        Self::new(Severity::Warning, message)
    }

    /// Advice: a note weaker than a warning.
    pub(crate) fn advice(message: String) -> Self {
        Self::new(Severity::Advice, message)
    }

    /// Set the machine-readable code.
    pub(crate) fn code(mut self, code: String) -> Self {
        self.0.code = Some(code);
        self
    }

    /// Set the closing fix-it hint.
    pub(crate) fn help(mut self, help: String) -> Self {
        self.0.help = Some(help);
        self
    }

    /// Attach the source these labels point into, by filename and text.
    pub(crate) fn source(mut self, filename: String, text: String) -> Self {
        self.0.source = Some(NamedSource::new(filename, text));
        self
    }

    /// Add a secondary labeled span.
    pub(crate) fn label(mut self, span: SourceSpan, text: String) -> Self {
        self.0.labels.push(LabeledSpan::new(
            Some(text),
            span.start,
            span.end - span.start,
        ));
        self
    }

    /// Add the primary labeled span: the one the snippet is framed around.
    /// At most one label should be primary.
    pub(crate) fn label_primary(mut self, span: SourceSpan, text: String) -> Self {
        self.0.labels.push(LabeledSpan::new_primary_with_span(
            Some(text),
            (span.start, span.end - span.start),
        ));
        self
    }

    /// Add a related sub-diagnostic, rendered as its own block.
    pub(crate) fn related(mut self, related: CompilerError) -> Self {
        self.0.related.push(related.0);
        self
    }
}

// -- Consumption (public) --

impl CompilerError {
    /// Render for humans: unicode box-drawing, colored when the output is a
    /// terminal and monochrome otherwise (honoring `NO_COLOR`). This is what
    /// [`Display`](fmt::Display) uses.
    pub fn render(&self) -> String {
        self.0.render_themed(GraphicalTheme::default())
    }

    /// Render with color and unicode box-drawing unconditionally.
    pub fn render_pretty(&self) -> String {
        self.0.render_themed(GraphicalTheme::unicode())
    }

    /// Render as monochrome ASCII: deterministic, terminal-independent, and the
    /// right choice for logs and test snapshots.
    pub fn render_plain(&self) -> String {
        self.0.render_themed(GraphicalTheme::none())
    }
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl std::error::Error for CompilerError {}

impl Diag {
    fn render_themed(&self, theme: GraphicalTheme) -> String {
        let mut out = String::new();
        // Writing to a String is infallible.
        let _ = GraphicalReportHandler::new_themed(theme).render_report(&mut out, self);
        out
    }
}

impl fmt::Display for Diag {
    // The bare headline only. Whole-report rendering is on `CompilerError`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Diag {}

impl Diagnostic for Diag {
    fn severity(&self) -> Option<Severity> {
        Some(self.severity)
    }

    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.code
            .as_deref()
            .map(|c| Box::new(c) as Box<dyn fmt::Display + 'a>)
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.help
            .as_deref()
            .map(|h| Box::new(h) as Box<dyn fmt::Display + 'a>)
    }

    fn source_code(&self) -> Option<&dyn SourceCode> {
        self.source.as_ref().map(|s| s as &dyn SourceCode)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        if self.labels.is_empty() {
            return None;
        }
        Some(Box::new(self.labels.clone().into_iter()))
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        if self.related.is_empty() {
            return None;
        }
        Some(Box::new(self.related.iter().map(|d| d as &dyn Diagnostic)))
    }
}

/// The diagnostics produced by a failed compilation.
///
/// Held on the error channel of the compiler's result. Ordinarily a single hard
/// error, but the type is plural so a pass can report several at once.
#[derive(Clone, Debug, Default)]
pub struct CompilerErrors(Vec<CompilerError>);

// -- Construction (crate-internal) --

impl CompilerErrors {
    /// An empty set.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Append a diagnostic.
    pub(crate) fn push(&mut self, error: CompilerError) {
        self.0.push(error);
    }
}

// -- Consumption (public) --

impl CompilerErrors {
    /// Whether the set holds no diagnostics.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The number of diagnostics.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Iterate over the diagnostics.
    pub fn iter(&self) -> std::slice::Iter<'_, CompilerError> {
        self.0.iter()
    }

    /// Render every diagnostic for humans (see [`CompilerError::render`]),
    /// back to back. This is what [`Display`](fmt::Display) uses.
    pub fn render(&self) -> String {
        self.render_each(CompilerError::render)
    }

    /// Render every diagnostic with color (see
    /// [`CompilerError::render_colored`]), back to back.
    pub fn render_pretty(&self) -> String {
        self.render_each(CompilerError::render_pretty)
    }

    /// Render every diagnostic as monochrome ASCII (see
    /// [`CompilerError::render_plain`]), back to back.
    pub fn render_plain(&self) -> String {
        self.render_each(CompilerError::render_plain)
    }

    fn render_each(&self, render: impl Fn(&CompilerError) -> String) -> String {
        self.0.iter().map(render).collect::<Vec<_>>().join("\n")
    }
}

impl IntoIterator for CompilerErrors {
    type Item = CompilerError;
    type IntoIter = std::vec::IntoIter<CompilerError>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl fmt::Display for CompilerErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl std::error::Error for CompilerErrors {}
