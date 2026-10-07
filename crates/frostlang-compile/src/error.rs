//! Compiler diagnostics.
//!
//! A [`Diagnostic`] is one diagnostic: a message, a severity, and any number
//! of labeled spans into a source, plus optional related sub-diagnostics that
//! render as their own separate blocks (the "this is wrong because of that over
//! there" shape, where "that" may even live in a different file). Because a
//! diagnostic carries its severity, the same type expresses both hard errors
//! and warnings.
//!
//! [`Diagnostics`] is the plural: the set returned on the error channel when
//! compilation fails.

#![allow(unused)] // Some builders have no consumer yet
// Silence, Clippy

#[cfg(test)]
mod demo;

use std::fmt;

use frostlang_parse::ParseError;
use frostlang_parse::ast::SourceSpan;
use miette::{
    Diagnostic as MietteDiagnostic, LabeledSpan, NamedSource, NarratableReportHandler, Severity,
    SourceCode,
};
#[cfg(feature = "graphical-diagnostics")]
use miette::{GraphicalReportHandler, GraphicalTheme};

/// A single compiler diagnostic: an error, a warning, or advice.
///
/// Constructed by the compiler; consume one by rendering it,
/// through [`Display`](fmt::Display), the same as [`render`](Self::render), or as
/// plain text with [`render_narrated`](Self::render_narrated). With the
/// [`graphical-diagnostics`](crate#features) feature, `render_pretty`,
/// `render_unicode`, and `render_plain` draw it in a fixed style.
#[derive(Clone, Debug)]
pub struct Diagnostic(Diag);

/// The data behind a [`Diagnostic`], and its `miette` view.
///
/// This is the type that implements [`MietteDiagnostic`]. Its [`Display`](fmt::Display)
/// is the bare headline: the graphical handler uses it for the block header, so
/// it must not itself invoke rendering. The pretty, whole-report rendering lives
/// on [`Diagnostic`], keeping the two roles on distinct types.
#[derive(Clone, Debug)]
struct Diag {
    severity: Severity,
    message: String,
    /// A short name for the kind of diagnostic, e.g. `unbound name`.
    code: Option<String>,
    /// A closing hint on how to fix the problem.
    help: Option<String>,
    /// The source these labels point into.
    ///
    /// A related diagnostic may leave this `None` to inherit its parent's
    /// source; set it when the diagnostic points into a different source.
    source: Option<NamedSource<String>>,
    /// Labeled spans, all resolved against `source`. A primary label (see
    /// [`Diagnostic::label_primary`]) frames the snippet.
    labels: Vec<LabeledSpan>,
    /// Sub-diagnostics, each rendered as its own block.
    related: Vec<Diag>,
}

// -- Construction (crate-internal) --

impl Diagnostic {
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

    /// Set the code naming the kind of diagnostic.
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
    pub(crate) fn related(mut self, related: Diagnostic) -> Self {
        self.0.related.push(related.0);
        self
    }

    /// Lift `error`, from parsing `source` as `filename`, into a compiler
    /// diagnostic, so that parse and compile failures can be reported alike.
    pub fn from_parse_error(error: &ParseError, filename: &str, source: &str) -> Self {
        // The parser stops at its first error, so this is one diagnostic. Its
        // first label is primary; the rest add context.
        let mut diagnostic = Self::error(error.message().to_string())
            .source(filename.to_string(), source.to_string());
        for (i, label) in error.labels().iter().enumerate() {
            diagnostic = if i == 0 {
                diagnostic.label_primary(label.span, label.text.clone())
            } else {
                diagnostic.label(label.span, label.text.clone())
            };
        }
        match error.help() {
            Some(help) => diagnostic.help(help.to_owned()),
            None => diagnostic,
        }
    }
}

// -- Consumption (public) --

impl Diagnostic {
    /// Render for humans. This is what [`Display`](fmt::Display) uses.
    ///
    /// With the [`graphical-diagnostics`](crate#features) feature, it draws each
    /// source snippet with its labels, adapting to the output: unicode and color
    /// at a terminal (monochrome under `NO_COLOR`), monochrome ASCII otherwise.
    /// Without it, it is [`render_narrated`](Self::render_narrated).
    pub fn render(&self) -> String {
        #[cfg(feature = "graphical-diagnostics")]
        return self.0.render_themed(GraphicalTheme::default());
        #[cfg(not(feature = "graphical-diagnostics"))]
        return self.render_narrated();
    }

    /// Render as narrated plain text, drawing nothing: the message, then the
    /// source lines with each label's line and columns, then any help. It reads
    /// well to a screen reader, and anywhere a drawing would not survive.
    pub fn render_narrated(&self) -> String {
        let mut out = String::new();
        // Writing to a String is infallible.
        let _ = NarratableReportHandler::new().render_report(&mut out, &self.0);
        out
    }

    /// Render with color and unicode box-drawing unconditionally.
    #[cfg(feature = "graphical-diagnostics")]
    pub fn render_pretty(&self) -> String {
        self.0.render_themed(GraphicalTheme::unicode())
    }

    /// Render with unicode box-drawing but no color: free of terminal escapes,
    /// so it reads well wherever the text ends up.
    #[cfg(feature = "graphical-diagnostics")]
    pub fn render_unicode(&self) -> String {
        self.0.render_themed(GraphicalTheme::unicode_nocolor())
    }

    /// Render as monochrome ASCII: deterministic, terminal-independent, and the
    /// right choice for logs and test snapshots.
    #[cfg(feature = "graphical-diagnostics")]
    pub fn render_plain(&self) -> String {
        self.0.render_themed(GraphicalTheme::none())
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl std::error::Error for Diagnostic {}

impl Diag {
    #[cfg(feature = "graphical-diagnostics")]
    fn render_themed(&self, theme: GraphicalTheme) -> String {
        let mut out = String::new();
        // Writing to a String is infallible.
        let _ = GraphicalReportHandler::new_themed(theme).render_report(&mut out, self);
        out
    }
}

impl fmt::Display for Diag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Diag {}

impl MietteDiagnostic for Diag {
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

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn MietteDiagnostic> + 'a>> {
        if self.related.is_empty() {
            return None;
        }
        Some(Box::new(
            self.related.iter().map(|d| d as &dyn MietteDiagnostic),
        ))
    }
}

/// The diagnostics produced by a failed compilation.
///
/// Held on the error channel of the compiler's result. Ordinarily a single hard
/// error, but the type is plural so a pass can report several at once.
#[derive(Clone, Debug, Default)]
pub struct Diagnostics(Vec<Diagnostic>);

// -- Construction (crate-internal) --

impl Diagnostics {
    /// An empty set.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Append a diagnostic.
    pub(crate) fn push(&mut self, diagnostic: Diagnostic) {
        self.0.push(diagnostic);
    }
}

impl From<Diagnostic> for Diagnostics {
    /// A single diagnostic is a set of one: lets a leaf `Diagnostic`
    /// propagate through `?` where a `Diagnostics` is expected.
    fn from(diagnostic: Diagnostic) -> Self {
        Self(vec![diagnostic])
    }
}

// -- Consumption (public) --

impl Diagnostics {
    /// Whether the set holds no diagnostics.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The number of diagnostics.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Iterate over the diagnostics.
    pub fn iter(&self) -> std::slice::Iter<'_, Diagnostic> {
        self.0.iter()
    }

    /// Render every diagnostic for humans (see [`Diagnostic::render`]),
    /// back to back. This is what [`Display`](fmt::Display) uses.
    pub fn render(&self) -> String {
        self.render_each(Diagnostic::render)
    }

    /// Render every diagnostic as narrated plain text (see
    /// [`Diagnostic::render_narrated`]), back to back.
    pub fn render_narrated(&self) -> String {
        self.render_each(Diagnostic::render_narrated)
    }

    /// Render every diagnostic with color and unicode (see
    /// [`Diagnostic::render_pretty`]), back to back.
    #[cfg(feature = "graphical-diagnostics")]
    pub fn render_pretty(&self) -> String {
        self.render_each(Diagnostic::render_pretty)
    }

    /// Render every diagnostic with unicode but no color (see
    /// [`Diagnostic::render_unicode`]), back to back.
    #[cfg(feature = "graphical-diagnostics")]
    pub fn render_unicode(&self) -> String {
        self.render_each(Diagnostic::render_unicode)
    }

    /// Render every diagnostic as monochrome ASCII (see
    /// [`Diagnostic::render_plain`]), back to back.
    #[cfg(feature = "graphical-diagnostics")]
    pub fn render_plain(&self) -> String {
        self.render_each(Diagnostic::render_plain)
    }

    fn render_each(&self, render: impl Fn(&Diagnostic) -> String) -> String {
        self.0.iter().map(render).collect::<Vec<_>>().join("\n")
    }
}

impl IntoIterator for Diagnostics {
    type Item = Diagnostic;
    type IntoIter = std::vec::IntoIter<Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl std::error::Error for Diagnostics {}
