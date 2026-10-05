//! Metacommands: instructions to the REPL or its frontend, typed in place of
//! Frost source.

use std::fmt;
use std::io;

/// A metacommand as typed: `:` and its name, then whatever argument follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    name: String,
    argument: String,
}

impl Invocation {
    /// The metacommand `segment` invokes, or `None` if it is not one: a
    /// metacommand starts with `:` and then its name, with no space between.
    ///
    /// ```
    /// use frostlang_repl::Invocation;
    ///
    /// let invocation = Invocation::parse(":undef x y").unwrap();
    /// assert_eq!((invocation.name(), invocation.argument()), ("undef", "x y"));
    /// assert_eq!(Invocation::parse(": undef x"), None);
    /// ```
    pub fn parse(segment: &str) -> Option<Self> {
        let rest = segment.strip_prefix(':')?;
        if rest.chars().next().is_none_or(char::is_whitespace) {
            return None;
        }
        let (name, argument) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        Some(Self {
            name: name.to_string(),
            argument: argument.trim().to_string(),
        })
    }

    /// The metacommand's name, without its `:`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Everything after the name, without the spaces around it: empty if
    /// nothing follows. It may span lines.
    pub fn argument(&self) -> &str {
        &self.argument
    }
}

/// A metacommand a [`Frontend`](crate::Frontend) adds: its name and how
/// `:help` describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetacommandSpec {
    name: String,
    arguments: String,
    summary: String,
}

impl MetacommandSpec {
    /// `:name`, described in `:help` by `summary`.
    ///
    /// A name is lowercase ASCII letters, digits, and `-`, starting with a
    /// letter.
    pub fn new(name: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: String::new(),
            summary: summary.into(),
        }
    }

    /// Describe the argument it takes, as `:help` shows it after the name:
    /// `<name>...`, say.
    pub fn with_arguments(mut self, arguments: impl Into<String>) -> Self {
        self.arguments = arguments.into();
        self
    }

    /// The name, without its `:`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The argument it takes, as `:help` shows it.
    pub fn arguments(&self) -> &str {
        &self.arguments
    }

    /// What it does, as `:help` shows it.
    pub fn summary(&self) -> &str {
        &self.summary
    }
}

/// What runs a metacommand in a [`MetacommandTable`].
pub type MetacommandHandler<F> = fn(&mut F, &Invocation) -> io::Result<()>;

/// A frontend's metacommands, each with the function that runs it.
///
/// A table keeps each name beside its handler, so a frontend implements
/// [`Frontend::metacommands`](crate::Frontend::metacommands) and
/// [`Frontend::metacommand`](crate::Frontend::metacommand) by deferring to it.
/// Keep the table apart from the frontend, as a `static`, so that a handler
/// may borrow the frontend mutably:
///
/// ```
/// use std::io;
/// use std::sync::LazyLock;
///
/// use frostlang_repl::{Invocation, MetacommandSpec, MetacommandTable};
///
/// struct Console {
///     greeting: String,
/// }
///
/// static METACOMMANDS: LazyLock<MetacommandTable<Console>> = LazyLock::new(|| {
///     MetacommandTable::new().with(
///         MetacommandSpec::new("greet", "Change the greeting").with_arguments("<greeting>"),
///         |console, invocation| {
///             console.greeting = invocation.argument().to_string();
///             Ok(())
///         },
///     )
/// });
///
/// let mut console = Console { greeting: String::new() };
/// let invocation = Invocation::parse(":greet hello").unwrap();
/// METACOMMANDS.dispatch(&mut console, &invocation).unwrap();
/// assert_eq!(console.greeting, "hello");
/// ```
pub struct MetacommandTable<F> {
    entries: Vec<(MetacommandSpec, MetacommandHandler<F>)>,
}

impl<F> Default for MetacommandTable<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F> MetacommandTable<F> {
    /// A table with no metacommands.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add the metacommand `spec` describes, run by `handler`.
    pub fn with(mut self, spec: MetacommandSpec, handler: MetacommandHandler<F>) -> Self {
        self.entries.push((spec, handler));
        self
    }

    /// Every metacommand in the table, in the order they were added.
    pub fn specs(&self) -> Vec<MetacommandSpec> {
        self.entries.iter().map(|(spec, _)| spec.clone()).collect()
    }

    /// Run `invocation` on `frontend` with the handler for its name. Fails if
    /// the table has no metacommand of that name.
    pub fn dispatch(&self, frontend: &mut F, invocation: &Invocation) -> io::Result<()> {
        match self
            .entries
            .iter()
            .find(|(spec, _)| spec.name() == invocation.name())
        {
            Some((_, handler)) => handler(frontend, invocation),
            None => Err(io::Error::other(format!(
                "no metacommand `:{}` in this table",
                invocation.name()
            ))),
        }
    }
}

/// A frontend's metacommand the REPL refuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidMetacommand {
    name: String,
    problem: MetacommandProblem,
}

/// Why the REPL refuses a frontend's metacommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetacommandProblem {
    /// Its name is not a valid metacommand name (see [`MetacommandSpec::new`]).
    InvalidName,
    /// It has the name of one of the REPL's own metacommands, which a frontend
    /// may not replace.
    Builtin,
    /// The frontend gives another metacommand the same name.
    Duplicate,
}

impl InvalidMetacommand {
    /// The refused metacommand's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Why it was refused.
    pub fn problem(&self) -> MetacommandProblem {
        self.problem
    }
}

impl fmt::Display for InvalidMetacommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let problem = match self.problem {
            MetacommandProblem::InvalidName => {
                "has an invalid name: use lowercase letters, digits, and `-`, starting with a letter"
            }
            MetacommandProblem::Builtin => "would replace a built-in metacommand",
            MetacommandProblem::Duplicate => "is given more than once",
        };
        write!(f, "the frontend's metacommand `:{}` {problem}", self.name)
    }
}

impl std::error::Error for InvalidMetacommand {}

/// Why a metacommand failed, the REPL's own or a frontend's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetacommandError {
    /// No metacommand has this name.
    Unknown(String),
    /// The metacommand cannot take the argument it was given.
    InvalidArgument {
        /// The metacommand's name, without its `:`.
        metacommand: String,
        /// What is wrong with the argument.
        reason: String,
    },
    /// The metacommand understood its argument, but could not carry it out.
    Failed {
        /// The metacommand's name, without its `:`.
        metacommand: String,
        /// What went wrong.
        reason: String,
    },
}

impl fmt::Display for MetacommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(name) => {
                write!(f, "there is no metacommand `:{name}`; `:help` lists them")
            }
            Self::InvalidArgument {
                metacommand,
                reason,
            }
            | Self::Failed {
                metacommand,
                reason,
            } => write!(f, "`:{metacommand}`: {reason}"),
        }
    }
}

impl std::error::Error for MetacommandError {}

/// One of the REPL's own metacommands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Builtin {
    Help,
    Quit,
    Bindings,
    Undef,
    Disassemble,
    Ast,
    Optimize,
}

impl Builtin {
    /// Every built-in, in the order `:help` lists them.
    const ALL: [Self; 7] = [
        Self::Help,
        Self::Quit,
        Self::Bindings,
        Self::Undef,
        Self::Disassemble,
        Self::Ast,
        Self::Optimize,
    ];

    fn spec(self) -> MetacommandSpec {
        let (arguments, summary) = match self {
            Self::Help => ("", "List the metacommands"),
            Self::Quit => ("", "End the session"),
            Self::Bindings => ("", "List the names bound, but not globals"),
            Self::Undef => ("<name>...", "Remove bindings"),
            Self::Disassemble => (
                "<source>",
                "Show the bytecode source compiles to, without running it",
            ),
            Self::Ast => ("<source>", "Show the syntax tree source parses to"),
            Self::Optimize => (
                "[<setting>, ...]",
                "Show the optimizations, or set them: `<name>=true|false`, `all`, or `none`",
            ),
        };
        MetacommandSpec::new(self.name(), summary).with_arguments(arguments)
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Help => "help",
            Self::Quit => "quit",
            Self::Bindings => "bindings",
            Self::Undef => "undef",
            Self::Disassemble => "disassemble",
            Self::Ast => "ast",
            Self::Optimize => "optimize",
        }
    }
}

/// Who runs a metacommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Handler {
    Builtin(Builtin),
    Frontend,
}

/// Every metacommand a session knows: the REPL's own, then its frontend's.
pub(crate) struct Registry {
    frontend: Vec<MetacommandSpec>,
}

impl Registry {
    /// A registry adding the frontend's `specs` to the built-ins, or the first
    /// of them the REPL refuses.
    pub(crate) fn new(specs: Vec<MetacommandSpec>) -> Result<Self, InvalidMetacommand> {
        for (at, spec) in specs.iter().enumerate() {
            let problem = if !is_valid_name(spec.name()) {
                Some(MetacommandProblem::InvalidName)
            } else if Builtin::ALL
                .iter()
                .any(|builtin| builtin.name() == spec.name())
            {
                Some(MetacommandProblem::Builtin)
            } else if specs[..at]
                .iter()
                .any(|earlier| earlier.name() == spec.name())
            {
                Some(MetacommandProblem::Duplicate)
            } else {
                None
            };
            if let Some(problem) = problem {
                return Err(InvalidMetacommand {
                    name: spec.name().to_string(),
                    problem,
                });
            }
        }
        Ok(Self { frontend: specs })
    }

    /// Who runs the metacommand `name`, if anyone does.
    pub(crate) fn find(&self, name: &str) -> Option<Handler> {
        if let Some(builtin) = Builtin::ALL
            .into_iter()
            .find(|builtin| builtin.name() == name)
        {
            return Some(Handler::Builtin(builtin));
        }
        self.frontend
            .iter()
            .any(|spec| spec.name() == name)
            .then_some(Handler::Frontend)
    }

    /// Every metacommand, one to a line, with its arguments and summary in
    /// aligned columns.
    pub(crate) fn help(&self) -> String {
        let specs: Vec<MetacommandSpec> = Builtin::ALL
            .into_iter()
            .map(Builtin::spec)
            .chain(self.frontend.iter().cloned())
            .collect();
        let usages: Vec<String> = specs
            .iter()
            .map(|spec| match spec.arguments() {
                "" => format!(":{}", spec.name()),
                arguments => format!(":{} {arguments}", spec.name()),
            })
            .collect();
        let width = usages.iter().map(String::len).max().unwrap_or(0);
        usages
            .iter()
            .zip(&specs)
            .map(|(usage, spec)| format!("{usage:width$}  {}", spec.summary()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
