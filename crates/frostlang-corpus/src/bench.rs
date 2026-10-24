//! Benchmarks: the corpus scripts that mark themselves with a `bench` field.
//! The `corpus` bench target times them; see `corpus/README.md`.

use std::path::Path;
use std::sync::Arc;
use std::{fs, io};

use frostlang::compile::{
    CompilerOptions, CompilerOutput, Optimization, OptimizationOptions, compile_program,
};
use frostlang::{Closure, Importer, Value, Vm, VmRuntimeConfiguration};

use crate::discover::{Entry, discover};
use crate::expectation::Expectation;
use crate::mode::Mode;
use crate::observe::inputs_importer;
use crate::verdict::{Report, check};

/// The corpus area whose scripts may be benchmarks, as a name prefix.
pub const BENCHMARKS: &str = "benchmarks/";

/// The environment variable selecting the compiler options to benchmark under.
pub const OPTIONS_VARIABLE: &str = "FROST_BENCH_OPTIONS";

/// The longest a part of a benchmark's ID may be. Criterion keeps each part as
/// a directory under its output, cut to this many bytes, so a longer part
/// could share one benchmark's history with another.
pub const MAX_ID_PART: usize = 64;

/// A corpus script marked as a benchmark, ready to compile and run with its
/// benchmark inputs.
pub struct Benchmark {
    /// The script.
    pub entry: Entry,
    source: String,
    importer: Arc<Importer>,
}

/// The compiler options a benchmark runs under, and the label naming them in
/// its ID.
#[derive(Clone, Debug, PartialEq)]
pub struct OptionSet {
    pub label: String,
    pub options: CompilerOptions,
}

impl Benchmark {
    /// Every benchmark under the corpus `root`, in name order. A script whose
    /// expectation cannot be read is not a benchmark; [`prepare`] refuses to
    /// run while one sits under [`BENCHMARKS`].
    pub fn discover(root: &Path) -> io::Result<Vec<Self>> {
        let mut benchmarks = Vec::new();
        for entry in discover(root)? {
            if !entry.name.starts_with(BENCHMARKS) {
                continue;
            }
            let source = fs::read_to_string(&entry.path)?;
            let Ok(Expectation {
                bench: Some(bench), ..
            }) = Expectation::read(&source)
            else {
                continue;
            };
            benchmarks.push(Self {
                entry,
                source,
                importer: inputs_importer(&bench.inputs),
            });
        }
        Ok(benchmarks)
    }

    /// The group its ID begins with: the script's directory.
    pub fn group(&self) -> &str {
        self.entry
            .name
            .rsplit_once('/')
            .map_or("", |(directory, _)| directory)
    }

    /// The part of its ID after the [`group`](Self::group): the script's file
    /// name, without its extension.
    pub fn file(&self) -> &str {
        self.entry
            .name
            .rsplit_once('/')
            .map_or(&self.entry.name, |(_, file)| file)
    }

    /// Compile the script under `options`: its output, or its diagnostics.
    pub fn compile(&self, options: CompilerOptions) -> Result<CompilerOutput, String> {
        compile_program(&format!("{}.frst", self.entry.name), &self.source, options)
            .map_err(|errors| errors.render_narrated())
    }

    /// The script, compiled under `options`, ready to [`run`](Self::run).
    pub fn closure(&self, options: CompilerOptions) -> Result<Arc<Closure>, String> {
        let output = self.compile(options)?;
        Ok(output
            .code
            .into_closure()
            .expect("a script compiled on its own captures nothing"))
    }

    /// Run `closure` with the benchmark inputs, discarding what it prints: its
    /// tail value, or the message of the error it raised.
    pub fn run(&self, closure: &Arc<Closure>) -> Result<Value, String> {
        let configuration =
            VmRuntimeConfiguration::default().with_print_sink(Arc::new(|_: &str| {}));
        Vm::factory()
            .configuration(configuration)
            .with_importer(Arc::clone(&self.importer))
            .build(Arc::clone(closure))
            .run()
            .map(|result| result.tail().clone())
            .map_err(|error| error.into_error().message().into_owned())
    }

    /// Why this benchmark cannot be timed under `option_sets`, if it cannot:
    /// an ID part longer than [`MAX_ID_PART`], or a failure to compile, to
    /// complete with its bench inputs, or to agree across the option sets.
    fn problem(&self, option_sets: &[OptionSet]) -> Option<String> {
        let name = &self.entry.name;
        let parts = [self.group(), self.file()]
            .into_iter()
            .map(str::to_string)
            .chain(
                option_sets
                    .iter()
                    .map(|set| format!("compile/{}", set.label)),
            );
        if let Some(part) = parts.into_iter().find(|part| part.len() > MAX_ID_PART) {
            return Some(format!(
                "{name}: its ID part `{part}` is longer than {MAX_ID_PART} bytes"
            ));
        }
        let mut results = Vec::new();
        for set in option_sets {
            let result = self
                .closure(set.options)
                .and_then(|closure| self.run(&closure));
            match result {
                Ok(value) => results.push((&set.label, value)),
                Err(message) => {
                    return Some(format!(
                        "{name}, under {}, with its bench inputs: {message}",
                        set.label
                    ));
                }
            }
        }
        let (first_label, first) = results.first()?;
        results.iter().find_map(|(label, value)| {
            (value != first).then(|| {
                format!(
                    "{name}, with its bench inputs: {first_label} gives {}, but {label} gives {}",
                    first.to_debug_string(),
                    value.to_debug_string()
                )
            })
        })
    }
}

/// The benchmarks under the corpus `root`, ready to time under `option_sets`,
/// or why none may be timed.
///
/// Timing a program that does the wrong thing measures nothing worth having,
/// so every script under [`BENCHMARKS`] must pass its corpus check, and every
/// benchmark must compile, complete with its bench inputs, and give the same
/// result under each option set. Its ID's parts must also each fit within
/// [`MAX_ID_PART`].
pub fn prepare(root: &Path, option_sets: &[OptionSet]) -> Result<Vec<Benchmark>, String> {
    let failed = |problems: Vec<String>| {
        Err(format!(
            "these benchmarks fail, so none is timed:\n{}",
            problems.join("\n")
        ))
    };
    let entries = discover(root).map_err(|error| error.to_string())?;
    let problems: Vec<String> = entries
        .iter()
        .filter(|entry| entry.name.starts_with(BENCHMARKS))
        .filter_map(|entry| match check(entry, Mode::Default).report {
            Report::Passed => None,
            report => Some(format!("{}: {report:?}", entry.name)),
        })
        .collect();
    if !problems.is_empty() {
        return failed(problems);
    }
    let benchmarks = Benchmark::discover(root).map_err(|error| error.to_string())?;
    let problems: Vec<String> = benchmarks
        .iter()
        .filter_map(|benchmark| benchmark.problem(option_sets))
        .collect();
    if !problems.is_empty() {
        return failed(problems);
    }
    Ok(benchmarks)
}

/// The option sets to benchmark under, from [`OPTIONS_VARIABLE`]'s value: the
/// default options alone when it is absent or blank.
///
/// Otherwise the value is `each`, for the default options, no optimization,
/// and the defaults without each optimization in turn; or a `;`-separated list
/// of option sets, each `default` or optimization settings applied over the
/// defaults, as [`OptimizationOptions::with_settings`] reads them. A set's
/// label is its text with spacing normalized, so that the same settings always
/// name the same benchmark.
pub fn option_sets(value: Option<&str>) -> Result<Vec<OptionSet>, String> {
    let defaults = CompilerOptions::new();
    let set = |label: String, optimization: OptimizationOptions| OptionSet {
        label,
        options: defaults.with_optimization(optimization),
    };
    let default = || set("default".to_string(), defaults.optimization_options);
    let value = value.map(str::trim).unwrap_or_default();
    if value.is_empty() {
        return Ok(vec![default()]);
    }
    if value == "each" {
        let base = defaults.optimization_options;
        let mut sets = vec![
            default(),
            set("none".to_string(), OptimizationOptions::NONE),
        ];
        sets.extend(Optimization::ALL.iter().map(|&optimization| {
            set(
                format!("{}=false", optimization.name()),
                base.with(optimization, false),
            )
        }));
        return Ok(sets);
    }
    let error = |message: String| format!("{OPTIONS_VARIABLE}: {message}");
    let mut sets: Vec<OptionSet> = Vec::new();
    for text in value.split(';').map(str::trim) {
        let next = match text {
            "" => return Err(error("an option set is empty".to_string())),
            "each" => return Err(error("`each` must stand alone".to_string())),
            "default" => default(),
            settings => {
                let optimization = defaults
                    .optimization_options
                    .with_settings(settings)
                    .map_err(|invalid| error(invalid.to_string()))?;
                set(normalized(settings), optimization)
            }
        };
        if let Some(same) = sets.iter().find(|set| set.options == next.options) {
            return Err(error(format!(
                "`{}` and `{}` select the same options",
                same.label, next.label
            )));
        }
        sets.push(next);
    }
    Ok(sets)
}

/// `settings` as a label: each setting trimmed, with no spaces around `=`, and
/// joined by `, `.
fn normalized(settings: &str) -> String {
    settings
        .split(',')
        .map(|setting| {
            setting
                .split('=')
                .map(str::trim)
                .collect::<Vec<_>>()
                .join("=")
        })
        .collect::<Vec<_>>()
        .join(", ")
}
