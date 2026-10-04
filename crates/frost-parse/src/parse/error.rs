use std::error::Error;
use std::fmt::{self, Display};

use miette::{LabeledSpan, MietteDiagnostic, NamedSource, Report};

use crate::ast::SourceSpan;

/// A labeled source span within a [`ParseError`].
/// Spans are absolute byte offsets into the original source.
#[derive(Clone, Debug)]
pub struct Label {
    /// The labeled source range.
    pub span: SourceSpan,
    /// The text shown at the span.
    pub text: String,
}

/// A diagnostic as it flows through parsing: a message plus zero or more
/// labeled spans, with no source text attached. The source is attached exactly
/// once, at the parse boundary, when the diagnostic is rendered into a
/// [`ParseError`]. Keeping the source out of the diagnostic avoids copying the
/// whole script on every error and lets sub-parsers (e.g. format-string
/// interpolations) propagate labels in whole-source coordinates.
#[derive(Clone, Debug)]
pub(crate) struct Diagnostic {
    message: String,
    labels: Vec<Label>,
    help: Option<String>,
}

impl Diagnostic {
    /// A diagnostic with a headline message and no labels yet. The message is
    /// the required primary line (shown next to miette's `x`); labels are
    /// attached with [`Diagnostic::with_label`].
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            labels: Vec::new(),
            help: None,
        }
    }

    /// A diagnostic with a headline message and a single labeled span:
    /// the common case for a localized parser error.
    pub(crate) fn at(
        message: impl Into<String>,
        span: SourceSpan,
        label: impl Into<String>,
    ) -> Self {
        Self::new(message).with_label(span, label)
    }

    /// Attach a labeled span. The first label attached is the primary location
    /// (see [`Diagnostic::primary_span`]); later ones add context.
    pub(crate) fn with_label(mut self, span: SourceSpan, text: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            text: text.into(),
        });
        self
    }

    /// Attach help: advice on what to write instead, shown below the labeled source.
    /// Absent help leaves the diagnostic as it is.
    pub(crate) fn with_help(mut self, help: Option<String>) -> Self {
        if help.is_some() {
            self.help = help;
        }
        self
    }

    /// The primary, human-readable message.
    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    /// All labeled spans, in the order they were attached.
    pub(crate) fn labels(&self) -> &[Label] {
        &self.labels
    }

    /// The primary source span (the first label), or an empty span if none.
    pub(crate) fn primary_span(&self) -> SourceSpan {
        self.labels.first().map(|l| l.span).unwrap_or_default()
    }
}

/// A parse error: a message plus labeled source spans, and a pretty-printed report of them.
/// Inspect it with [`message`](Self::message) and [`labels`](Self::labels);
/// display it with [`rendered`](Self::rendered) or `Display`.
#[derive(Clone, Debug)]
pub struct ParseError {
    diagnostic: Diagnostic,
    rendered: String,
}

impl ParseError {
    /// Render a diagnostic against the full source, attaching the source once.
    pub(crate) fn from_diag(diagnostic: Diagnostic, filename: &str, source: &str) -> Self {
        let labels: Vec<LabeledSpan> = diagnostic
            .labels
            .iter()
            .map(|l| LabeledSpan::at(l.span.start..l.span.end, l.text.clone()))
            .collect();

        let mut report = MietteDiagnostic::new(diagnostic.message.clone()).with_labels(labels);
        if let Some(help) = &diagnostic.help {
            report = report.with_help(help.clone());
        }
        let report =
            Report::new(report).with_source_code(NamedSource::new(filename, source.to_owned()));

        ParseError {
            rendered: format!("{report:?}"),
            diagnostic,
        }
    }

    /// The primary, human-readable error message.
    pub fn message(&self) -> &str {
        self.diagnostic.message()
    }

    /// All labeled spans, in the order they were attached. The first is the
    /// primary location; later labels add context.
    pub fn labels(&self) -> &[Label] {
        self.diagnostic.labels()
    }

    /// The primary source span (the first label), or an empty span if none.
    pub fn primary_span(&self) -> SourceSpan {
        self.diagnostic.primary_span()
    }

    /// Advice on what to write instead, when the error looks like a habit
    /// carried over from another language.
    pub fn help(&self) -> Option<&str> {
        self.diagnostic.help.as_deref()
    }

    /// The pretty-formatted error string, intended for display.
    pub fn rendered(&self) -> &str {
        &self.rendered
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rendered)
    }
}

impl Error for ParseError {}
