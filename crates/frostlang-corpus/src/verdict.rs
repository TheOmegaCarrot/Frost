use std::any::Any;
use std::collections::BTreeMap;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};

use frostlang::{Value, write_quoted_string};
use rayon::prelude::*;

use crate::bench::BENCHMARKS;
use crate::discover::Entry;
use crate::expectation::{Expectation, Expected, ExpectedRun, Kind, KnownIssue};
use crate::mode::Mode;
use crate::observe::{Finished, Observed, observe};

/// How a script fared.
#[derive(Debug)]
pub struct Verdict {
    /// The kind of outcome its directory declares, if it is in one that does.
    pub kind: Option<Kind>,
    pub report: Report,
}

/// What became of a script.
#[derive(Debug, PartialEq, Eq)]
pub enum Report {
    /// It behaved as expected.
    Passed,
    /// It did not behave as expected, for this reason.
    Failed(String),
    /// It panicked, which no script may do, whatever it expects.
    Panicked(String),
    /// It failed or panicked, as a known issue predicts, for this reason.
    KnownIssue { issue: KnownIssue, detail: String },
    /// It behaved as expected despite a known issue, so the marker is stale.
    Resolved(KnownIssue),
    /// The script or its expectation cannot be used, for this reason.
    Malformed(String),
}

impl Report {
    /// Whether the run counts this as a success.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Passed | Self::KnownIssue { .. })
    }

    /// Why this counts as a failure, if it does.
    pub fn failure(&self) -> Option<String> {
        match self {
            Self::Passed | Self::KnownIssue { .. } => None,
            Self::Failed(reason) => Some(reason.clone()),
            Self::Panicked(message) => Some(format!("PANIC: {message}")),
            Self::Resolved(issue) => Some(format!(
                "the script now behaves as expected, despite known issue {}: \
                 remove its `known_issue` marker",
                issue.task
            )),
            Self::Malformed(reason) => Some(format!("malformed script: {reason}")),
        }
    }
}

/// Check the script `entry` under each of `mode`'s options, as
/// [`check_source`] does.
pub fn check(entry: &Entry, mode: Mode) -> Verdict {
    match std::fs::read_to_string(&entry.path) {
        Ok(source) => check_source(&entry.name, &source, mode),
        Err(error) => Verdict {
            kind: declared_kind(&entry.name),
            report: Report::Malformed(format!("cannot read {}: {error}", entry.path.display())),
        },
    }
}

/// Check the script `source`, named `name` as [`Entry::name`] names it, under
/// each of `mode`'s options. The options run in parallel on the current rayon
/// thread pool.
pub fn check_source(name: &str, source: &str, mode: Mode) -> Verdict {
    let kind = declared_kind(name);
    Verdict {
        kind,
        report: report(name, source, kind, mode),
    }
}

/// The kind of outcome a script's directory, the last in its name, declares.
fn declared_kind(name: &str) -> Option<Kind> {
    let (directories, _) = name.rsplit_once('/')?;
    let directory = directories.rsplit('/').next()?;
    Kind::from_directory(directory)
}

fn report(name: &str, source: &str, kind: Option<Kind>, mode: Mode) -> Report {
    let Some(kind) = kind else {
        let directories = Kind::ALL.map(Kind::directory).join(", ");
        return Report::Malformed(format!(
            "a script must be in a directory named for its outcome: one of {directories}"
        ));
    };
    let expectation = match guarded(|| Expectation::read(source)) {
        Ok(Ok(expectation)) => expectation,
        Ok(Err(reason)) => return Report::Malformed(reason),
        Err(message) => return Report::Panicked(format!("reading the expectation: {message}")),
    };
    if expectation.bench.is_some() && !name.starts_with(BENCHMARKS) {
        return Report::Malformed(format!(
            "only a script under `{BENCHMARKS}` may have `bench`"
        ));
    }
    if expectation.expected.kind() != kind {
        return Report::Malformed(format!(
            "the expectation is of a {} script, but the script is in a `{}` directory",
            expectation.expected.kind().directory(),
            kind.directory()
        ));
    }
    let raw = run(name, source, &expectation, mode);
    match (expectation.known_issue, raw) {
        (None, raw) => raw,
        (Some(issue), Report::Passed) => Report::Resolved(issue),
        (Some(issue), Report::Failed(detail) | Report::Panicked(detail)) => {
            Report::KnownIssue { issue, detail }
        }
        (Some(_), other) => other,
    }
}

/// Run the script under each of `mode`'s options, in parallel on the current
/// rayon pool, requiring them to agree, and compare the outcome to
/// `expectation`.
fn run(name: &str, source: &str, expectation: &Expectation, mode: Mode) -> Report {
    let filename = format!("{name}.frst");
    let inputs = &expectation.inputs;
    let options = mode.options();
    let observations: Vec<_> = options
        .par_iter()
        .map(|&options| guarded(|| observe(&filename, source, inputs, options)))
        .collect();
    let mut runs = options.into_iter().zip(observations);
    let (baseline_options, baseline) = runs.next().expect("every mode has options");
    let baseline = match baseline {
        Ok(observed) => observed,
        Err(message) => return Report::Panicked(message),
    };
    for (other_options, observed) in runs {
        match observed {
            Ok(observed) if observed == baseline => {}
            Ok(observed) => {
                return Report::Failed(format!(
                    "optimization changed the outcome:\n\
                     under {baseline_options:?}:\n{baseline:#?}\n\
                     under {other_options:?}:\n{observed:#?}"
                ));
            }
            Err(message) => {
                return Report::Panicked(format!("under {other_options:?}: {message}"));
            }
        }
    }
    match compare(&expectation.expected, &baseline) {
        Ok(()) => Report::Passed,
        Err(reason) => Report::Failed(reason),
    }
}

/// Call `f`, catching a panic as its message.
fn guarded<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|payload| panic_message(payload.as_ref()))
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(ToString::to_string))
        .unwrap_or_else(|| "a panic with no message".to_string())
}

fn compare(expected: &Expected, observed: &Observed) -> Result<(), String> {
    match (expected, observed) {
        (Expected::Rejected(fragments), Observed::Rejected(diagnostics)) => {
            match_fragments("the error", fragments, &diagnostic_text(diagnostics))
        }
        (Expected::Compiles { .. }, Observed::Rejected(diagnostics)) => {
            Err(format!("expected it to compile, but:\n{diagnostics}"))
        }
        (Expected::Rejected(_), Observed::Ran { outcome, .. }) => Err(format!(
            "expected it not to compile, but it compiled, and {}",
            describe_outcome(outcome)
        )),
        (
            Expected::Compiles { lints, run },
            Observed::Ran {
                lints: actual_lints,
                outcome,
                printed,
            },
        ) => {
            let mismatches: Vec<String> = [
                compare_run(run, outcome, printed),
                compare_lints(lints, actual_lints),
            ]
            .into_iter()
            .filter_map(Result::err)
            .collect();
            if mismatches.is_empty() {
                Ok(())
            } else {
                Err(mismatches.join("\n"))
            }
        }
    }
}

/// Compare a run's `outcome`, and the text it `printed`, with the expected `run`.
fn compare_run(
    run: &ExpectedRun,
    outcome: &Result<Finished, String>,
    printed: &[String],
) -> Result<(), String> {
    match run {
        ExpectedRun::Raises(fragments) => match outcome {
            Err(message) => match_fragments("the error", fragments, message),
            Ok(_) => Err(format!(
                "expected it to raise, but {}",
                describe_outcome(outcome)
            )),
        },
        ExpectedRun::Completes {
            result,
            exports,
            printed: expected_printed,
        } => {
            let Ok(finished) = outcome else {
                return Err(format!(
                    "expected it to complete, but {}",
                    describe_outcome(outcome)
                ));
            };
            let mut mismatches = Vec::new();
            if finished.result != *result {
                mismatches.push(format!(
                    "result: expected {}, got {}",
                    show(result),
                    show(&finished.result)
                ));
            }
            if finished.exports != *exports {
                mismatches.push(format!(
                    "exports: expected {}, got {}",
                    show_exports(exports),
                    show_exports(&finished.exports)
                ));
            }
            if printed != expected_printed {
                mismatches.push(format!(
                    "printed: expected {expected_printed:?}, got {printed:?}"
                ));
            }
            if mismatches.is_empty() {
                Ok(())
            } else {
                Err(mismatches.join("\n"))
            }
        }
    }
}

/// Compare a script's `lints`, each narrated, with the `expected` ones: one entry
/// each, in source order, whose fragments that lint must contain.
fn compare_lints(expected: &[Vec<String>], lints: &[String]) -> Result<(), String> {
    let texts: Vec<String> = lints.iter().map(|lint| diagnostic_text(lint)).collect();
    if texts.len() != expected.len() {
        let listing: String = texts.iter().map(|text| format!("\n{text}")).collect();
        return Err(format!(
            "lints: expected {}, got {}{listing}",
            expected.len(),
            texts.len()
        ));
    }
    expected
        .iter()
        .zip(&texts)
        .enumerate()
        .try_for_each(|(index, (fragments, text))| {
            match_fragments(&format!("lint {}", index + 1), fragments, text)
        })
}

/// The text of narrated diagnostics that fragments match against: everything
/// but the echoed source lines and the lines naming the file, so a fragment
/// cannot match the script itself or its name.
fn diagnostic_text(narrated: &str) -> String {
    narrated
        .lines()
        .filter(|line| !line.starts_with("snippet line") && !line.starts_with("Begin snippet"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether `text`, that of `what` (as `the error` or `lint 2`), contains each of
/// `fragments`, ignoring case.
fn match_fragments(what: &str, fragments: &[String], text: &str) -> Result<(), String> {
    let lowered = text.to_lowercase();
    let missing: Vec<&String> = fragments
        .iter()
        .filter(|fragment| !lowered.contains(&fragment.to_lowercase()))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{what} does not mention {missing:?}; it reads:\n{text}"
        ))
    }
}

fn describe_outcome(outcome: &Result<Finished, String>) -> String {
    match outcome {
        Ok(finished) => format!("it completed with {}", show(&finished.result)),
        Err(message) => format!("it raised: {message}"),
    }
}

/// A value as Frost source would write it.
fn show(value: &Value) -> String {
    // `to_frost_string` leaves a top-level String unquoted.
    match value {
        Value::String(text) => {
            let mut quoted = String::new();
            write_quoted_string(text, &mut quoted).expect("writing to a String cannot fail");
            quoted
        }
        _ => value.to_frost_string(),
    }
}

fn show_exports(exports: &BTreeMap<String, Value>) -> String {
    let fields: Vec<String> = exports
        .iter()
        .map(|(name, value)| format!("{name}: {}", show(value)))
        .collect();
    format!("{{{}}}", fields.join(", "))
}

/// Counts of verdicts, for the summary printed after a run.
#[derive(Debug)]
pub struct Tally {
    mode: Mode,
    by_kind: BTreeMap<Kind, Counts>,
    resolved: Vec<String>,
    malformed: usize,
}

#[derive(Debug, Default)]
struct Counts {
    passed: usize,
    failed: usize,
    panicked: usize,
    known_issues: usize,
    regressions: usize,
}

impl Tally {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            by_kind: Kind::ALL.map(|kind| (kind, Counts::default())).into(),
            resolved: Vec::new(),
            malformed: 0,
        }
    }

    pub fn record(&mut self, name: &str, verdict: &Verdict) {
        let Some(counts) = verdict.kind.and_then(|kind| self.by_kind.get_mut(&kind)) else {
            self.malformed += 1;
            return;
        };
        match &verdict.report {
            Report::Passed => counts.passed += 1,
            Report::Failed(_) => counts.failed += 1,
            Report::Panicked(_) => counts.panicked += 1,
            Report::KnownIssue { issue, .. } => {
                counts.known_issues += 1;
                counts.regressions += usize::from(issue.regression);
            }
            Report::Resolved(issue) => self.resolved.push(format!("{name} ({})", issue.task)),
            Report::Malformed(_) => self.malformed += 1,
        }
    }
}

impl fmt::Display for Tally {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "corpus summary ({})", self.mode.describe())?;
        writeln!(
            f,
            "{:<16}{:>8}{:>8}{:>8}{:>14}{:>14}",
            "", "pass", "fail", "panic", "known issue", "(regression)"
        )?;
        for (kind, counts) in &self.by_kind {
            writeln!(
                f,
                "{:<16}{:>8}{:>8}{:>8}{:>14}{:>14}",
                kind.directory(),
                counts.passed,
                counts.failed,
                counts.panicked,
                counts.known_issues,
                counts.regressions
            )?;
        }
        writeln!(f, "malformed: {}", self.malformed)?;
        write!(
            f,
            "known issues now behaving as expected: {}",
            self.resolved.len()
        )?;
        for name in &self.resolved {
            write!(f, "\n    {name}")?;
        }
        Ok(())
    }
}
