use std::collections::BTreeMap;

use frostlang::compile::{CompilerOptions, compile_program};
use frostlang::{Value, Vm, from_value, is_identifier_like_and_not_keyword};
use serde::{Deserialize, Deserializer};

/// The prefix of each line of a script's expectation block.
const PREFIX: &str = "#=";

/// What a script is expected to do, read from its expectation block.
#[derive(Debug)]
pub struct Expectation {
    pub expected: Expected,
    /// Values the script reads by name, supplied when it runs, so that no
    /// compile-time analysis can know them.
    pub inputs: BTreeMap<String, Value>,
    /// The active bug that keeps the script from behaving as expected, if any.
    pub known_issue: Option<KnownIssue>,
    /// How to benchmark the script, if it is a benchmark.
    pub bench: Option<Bench>,
}

/// How a benchmark script is benchmarked.
#[derive(Debug)]
pub struct Bench {
    /// The inputs a benchmark run supplies: the script's own, or larger ones.
    pub inputs: BTreeMap<String, Value>,
}

/// The outcome a script is expected to have.
#[derive(Debug)]
pub enum Expected {
    /// It compiles, and its run ends as `run` expects.
    Compiles {
        /// Its lints, one entry each, in source order: the fragments that lint
        /// contains, ignoring case.
        lints: Vec<Vec<String>>,
        run: ExpectedRun,
    },
    /// It fails to compile, with diagnostics containing each fragment,
    /// ignoring case.
    Rejected(Vec<String>),
}

/// How the run of a script that compiles is expected to end.
#[derive(Debug)]
pub enum ExpectedRun {
    /// It completes.
    Completes {
        /// The tail value.
        result: Value,
        /// Every export, by name.
        exports: BTreeMap<String, Value>,
        /// The text of each `print`, in order.
        printed: Vec<String>,
    },
    /// It raises an error whose message contains each fragment, ignoring
    /// case.
    Raises(Vec<String>),
}

/// The kind of outcome a script is expected to have, which also names the
/// directory it lives in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Good,
    RuntimeError,
    CompileError,
}

/// An active bug, tracked by a Backlog task, that keeps a script from
/// behaving as expected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownIssue {
    /// The Backlog task's ID, such as `FROST-123`.
    pub task: String,
    /// Whether the bug is an accepted regression: the script once behaved as
    /// expected.
    pub regression: bool,
}

impl Expected {
    pub fn kind(&self) -> Kind {
        match self {
            Self::Compiles {
                run: ExpectedRun::Completes { .. },
                ..
            } => Kind::Good,
            Self::Compiles {
                run: ExpectedRun::Raises(_),
                ..
            } => Kind::RuntimeError,
            Self::Rejected(_) => Kind::CompileError,
        }
    }
}

impl Kind {
    pub const ALL: [Self; 3] = [Self::Good, Self::RuntimeError, Self::CompileError];

    /// The name of the directory holding scripts of this kind.
    pub fn directory(self) -> &'static str {
        match self {
            Self::Good => "good",
            Self::RuntimeError => "runtime-error",
            Self::CompileError => "compile-error",
        }
    }

    /// The kind whose scripts live in a directory named `name`.
    pub fn from_directory(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.directory() == name)
    }
}

impl Expectation {
    /// Read the expectation from `source`'s expectation block: its lines that
    /// begin with `#=`, which must be contiguous. Without the prefix (and one
    /// following space, if present), they are a Frost expression evaluating to
    /// a Map.
    pub fn read(source: &str) -> Result<Self, String> {
        let lines: Vec<(usize, &str)> = source
            .lines()
            .enumerate()
            .filter_map(|(number, line)| line.strip_prefix(PREFIX).map(|rest| (number, rest)))
            .collect();
        if lines.is_empty() {
            return Err(format!("there is no `{PREFIX}` expectation block"));
        }
        if lines.windows(2).any(|pair| pair[1].0 != pair[0].0 + 1) {
            return Err(format!(
                "the `{PREFIX}` lines must form one contiguous block"
            ));
        }
        let text = lines
            .iter()
            .map(|(_, line)| line.strip_prefix(' ').unwrap_or(line))
            .collect::<Vec<_>>()
            .join("\n");
        let fields: Fields = from_value(evaluate(&text)?)
            .map_err(|error| format!("the expectation is malformed: {}", error.message()))?;
        fields.validate()
    }
}

/// Evaluate an expectation block's Frost source.
fn evaluate(text: &str) -> Result<Value, String> {
    let output =
        compile_program("expectation", text, CompilerOptions::new()).map_err(|diagnostics| {
            format!(
                "the expectation does not compile:\n{}",
                diagnostics.render_narrated()
            )
        })?;
    let closure = output
        .code
        .into_closure()
        .map_err(|_| "the expectation uses a name it does not define".to_string())?;
    let result = Vm::factory().build(closure).run().map_err(|error| {
        format!(
            "the expectation raised an error: {}",
            error.into_error().message()
        )
    })?;
    Ok(result.tail().clone())
}

/// The expectation Map's fields, as written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fields {
    #[serde(default, deserialize_with = "present")]
    result: Option<Value>,
    exports: Option<BTreeMap<String, Value>>,
    printed: Option<Vec<String>>,
    raises: Option<Fragments>,
    rejects: Option<Fragments>,
    lints: Option<Vec<Fragments>>,
    known_issue: Option<String>,
    #[serde(default)]
    regression: bool,
    #[serde(default)]
    inputs: BTreeMap<String, Value>,
    /// Read as a plain value and only then by its shape: an untagged enum
    /// would buffer it in a form that cannot hold a Function input.
    bench: Option<Value>,
}

/// What the `bench` field may be.
const BENCH_FORMS: &str = "`bench` is `true` or `{inputs: {...}}`";

/// The Map form of the `bench` field.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BenchInputs {
    inputs: BTreeMap<String, Value>,
}

/// Deserializes a field that is present, even as Null, to `Some`; an absent
/// field is left to `#[serde(default)]`.
fn present<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}

/// One error-message fragment, or several that must all appear.
#[derive(Deserialize)]
#[serde(untagged)]
enum Fragments {
    One(String),
    Many(Vec<String>),
}

impl Fragments {
    fn validate(self, field: &str) -> Result<Vec<String>, String> {
        let fragments = match self {
            Self::One(fragment) => vec![fragment],
            Self::Many(fragments) => fragments,
        };
        if fragments.is_empty() || fragments.iter().any(|fragment| fragment.trim().is_empty()) {
            return Err(format!(
                "`{field}` must name at least one fragment, and no fragment may be blank"
            ));
        }
        Ok(fragments)
    }
}

impl Fields {
    fn validate(self) -> Result<Expectation, String> {
        let has_completion_fields = self.exports.is_some() || self.printed.is_some();
        let has_lints = self.lints.is_some();
        let lints = validate_lints(self.lints)?;
        let expected = match (self.result, self.raises, self.rejects) {
            (Some(result), None, None) => Expected::Compiles {
                lints,
                run: ExpectedRun::Completes {
                    result,
                    exports: self.exports.unwrap_or_default(),
                    printed: self.printed.unwrap_or_default(),
                },
            },
            (None, Some(fragments), None) => Expected::Compiles {
                lints,
                run: ExpectedRun::Raises(fragments.validate("raises")?),
            },
            (None, None, Some(fragments)) => Expected::Rejected(fragments.validate("rejects")?),
            _ => {
                return Err(
                    "the expectation must have exactly one of `result`, `raises`, and `rejects`"
                        .to_string(),
                );
            }
        };
        if has_completion_fields && expected.kind() != Kind::Good {
            return Err("only an expectation with `result` may have `exports` or `printed`".into());
        }
        if has_lints && expected.kind() == Kind::CompileError {
            return Err("only an expectation with `result` or `raises` may have `lints`".into());
        }
        let known_issue = match (self.known_issue, self.regression) {
            (Some(task), regression) if is_task_id(&task) => Some(KnownIssue { task, regression }),
            (Some(task), _) => {
                return Err(format!(
                    "`known_issue` must be a Backlog task ID, such as 'FROST-123', not {task:?}"
                ));
            }
            (None, true) => return Err("`regression` requires a `known_issue`".to_string()),
            (None, false) => None,
        };
        let bench = match self.bench {
            None => None,
            Some(_) if expected.kind() != Kind::Good => {
                return Err("only an expectation with `result` may have `bench`".into());
            }
            // A benchmark is timed only while it behaves as expected.
            Some(_) if known_issue.is_some() => {
                return Err("a benchmark may not have a `known_issue`".into());
            }
            Some(Value::Bool(true)) => Some(Bench {
                inputs: self.inputs.clone(),
            }),
            Some(Value::Bool(false)) => {
                return Err(format!("{BENCH_FORMS}; leave it out for none"));
            }
            Some(map @ Value::Map(_)) => {
                let BenchInputs { inputs } = from_value(map)
                    .map_err(|error| format!("{BENCH_FORMS}: {}", error.message()))?;
                Some(Bench { inputs })
            }
            Some(other) => return Err(format!("{BENCH_FORMS}, not {}", other.type_name())),
        };
        let mut input_names = self
            .inputs
            .keys()
            .chain(bench.iter().flat_map(|bench| bench.inputs.keys()));
        if let Some(name) = input_names.find(|name| !is_identifier_like_and_not_keyword(name)) {
            return Err(format!(
                "the input {name:?} is not a name a script can bind"
            ));
        }
        Ok(Expectation {
            expected,
            inputs: self.inputs,
            known_issue,
            bench,
        })
    }
}

/// The `lints` field's entries, each validated as fragments. Absent, the script
/// must produce no lints, so an empty list is redundant and rejected.
fn validate_lints(lints: Option<Vec<Fragments>>) -> Result<Vec<Vec<String>>, String> {
    let Some(lints) = lints else {
        return Ok(Vec::new());
    };
    if lints.is_empty() {
        return Err("`lints` must name at least one lint; leave it out for none".to_string());
    }
    lints
        .into_iter()
        .map(|fragments| fragments.validate("each entry of `lints`"))
        .collect()
}

/// Whether `id` has the shape of a Backlog task ID: `FROST-` and a number,
/// with any subtask numbers after dots, such as `FROST-12.3`.
fn is_task_id(id: &str) -> bool {
    id.strip_prefix("FROST-").is_some_and(|number| {
        number
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    })
}
