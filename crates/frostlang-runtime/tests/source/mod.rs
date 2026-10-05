//! Harness for the tests that start from Frost source: the globals and the
//! standard library, tested as scripts use them.
//!
//! A copy of the compiler tests' harness (`frostlang-compile/tests/common`), kept as
//! its own module beside this crate's bytecode helpers in `common`.
//!
//! The core of it is [`optimization_permutations`]: a [`Script`] compiles and
//! runs under many permutations of the optimization options, or every one with
//! `EXHAUSTIVE_COMPILER_TESTS` set, and its checks require them all to agree.
//! Optimization must never change what a program does, so most tests get that
//! assurance for free just by going through [`Script`].
//!
//! Runs are unmetered. Fuel use and call depth are the things optimization may
//! change (it may lower either, e.g. by folding or inlining a call), so under a
//! limit the permutations could legitimately disagree on whether a script
//! exceeds it. A [`Script::max_call_depth`] is only for scripts that stay far
//! under the limit whatever optimization does (such as tail recursion) or blow
//! far past it (such as unbounded recursion).
//!
//! Tests about what optimization *emits* select the permutations they apply to
//! and read the code with [`Script::code_where`].

// Each test binary uses its own subset of the harness.
#![allow(dead_code)]

pub(crate) mod assertions;

use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use frostlang_compile::{
    CompilerErrors, CompilerOptions, CompilerOutput, Optimization, OptimizationOptions,
    compile_in_scope,
};
use frostlang_runtime::{
    Bytecode, CompiledFunction, Importer, MapKey, Value, Vm, VmRuntimeConfiguration,
};

/// Every optimization off. A base for picking options explicitly:
/// `UNOPTIMIZED.with(Optimization::ConstantFold, true)`.
pub(crate) const UNOPTIMIZED: OptimizationOptions = OptimizationOptions::NONE;

/// The permutations of the optimization options a [`Script`] runs under,
/// starting with [`UNOPTIMIZED`].
///
/// By default, these are no optimization, every optimization, each one alone,
/// and all but each one: every optimization is checked on its own and beside
/// all the others, at a cost that grows with the number of options. With the
/// environment variable `EXHAUSTIVE_COMPILER_TESTS` set, to any value, they are
/// every permutation instead, at a cost that doubles with each option.
pub(crate) fn optimization_permutations() -> impl Iterator<Item = OptimizationOptions> {
    let count = Optimization::ALL.len();
    let all: u64 = (1 << count) - 1;
    let bits: Vec<u64> = if std::env::var_os("EXHAUSTIVE_COMPILER_TESTS").is_some() {
        (0..=all).collect()
    } else {
        let alone = (0..count).map(|index| 1 << index);
        let all_but = alone.clone().map(|bit| all & !bit);
        [0, all].into_iter().chain(alone).chain(all_but).collect()
    };
    bits.into_iter().map(options_from_bits)
}

/// The options with `Optimization::ALL[n]` on where bit `n` of `bits` is set.
fn options_from_bits(bits: u64) -> OptimizationOptions {
    Optimization::ALL
        .iter()
        .enumerate()
        .fold(UNOPTIMIZED, |options, (index, &optimization)| {
            options.with(optimization, bits & (1 << index) != 0)
        })
}

/// A completed run: the program's tail value and its exports.
#[derive(Debug, PartialEq)]
pub(crate) struct Finished {
    pub(crate) tail: Value,
    pub(crate) exports: BTreeMap<String, Value>,
}

/// All a run does: its outcome, and the text of each `print` it makes.
#[derive(Debug, PartialEq)]
struct Observed {
    outcome: Result<Finished, String>,
    printed: Vec<String>,
}

/// A script to compile and run, with the enclosing scope it compiles against.
pub(crate) struct Script {
    source: String,
    filename: String,
    // Names in the enclosing scope; a subset has values in `captures`.
    scope: Vec<String>,
    captures: BTreeMap<String, Value>,
    implicit_export: bool,
    max_call_depth: Option<NonZeroUsize>,
    importer: Arc<Importer>,
}

impl Script {
    pub(crate) fn new(source: &str) -> Self {
        Self {
            source: source.to_string(),
            filename: "test.frst".to_string(),
            scope: Vec::new(),
            captures: BTreeMap::new(),
            implicit_export: false,
            max_call_depth: None,
            importer: Arc::default(),
        }
    }

    /// Run with `importer` resolving imports; without one, every import fails.
    pub(crate) fn importer(mut self, importer: Arc<Importer>) -> Self {
        self.importer = importer;
        self
    }

    /// Run with the VM's call depth limited to `depth` frames.
    pub(crate) fn max_call_depth(mut self, depth: usize) -> Self {
        self.max_call_depth = Some(NonZeroUsize::new(depth).expect("a depth limit is positive"));
        self
    }

    pub(crate) fn filename(mut self, filename: &str) -> Self {
        self.filename = filename.to_string();
        self
    }

    /// Put `name` in the enclosing scope, with `value` supplied for it at close.
    pub(crate) fn capture(mut self, name: &str, value: Value) -> Self {
        self.scope.push(name.to_string());
        self.captures.insert(name.to_string(), value);
        self
    }

    /// [`capture`](Self::capture) each `(name, value)` pair.
    pub(crate) fn captures(self, captures: &[(&str, Value)]) -> Self {
        captures.iter().fold(self, |script, (name, value)| {
            script.capture(name, value.clone())
        })
    }

    /// Put `name` in the enclosing scope with no value supplied, which is only
    /// valid if the script never captures it.
    pub(crate) fn in_scope(mut self, name: &str) -> Self {
        self.scope.push(name.to_string());
        self
    }

    pub(crate) fn implicit_export(mut self) -> Self {
        self.implicit_export = true;
        self
    }

    /// The run's outcome, `Err` holding a runtime error's message. Every
    /// optimization permutation must compile the script and agree on this, and
    /// on what it prints.
    pub(crate) fn outcome(&self) -> Result<Finished, String> {
        self.observe().outcome
    }

    /// The text of each `print` the run makes, whether it completes or raises.
    /// Every permutation must agree on this, and on the outcome.
    pub(crate) fn printed(&self) -> Vec<String> {
        self.observe().printed
    }

    fn observe(&self) -> Observed {
        let mut permutations = optimization_permutations();
        let first = permutations
            .next()
            .expect("there is always one permutation");
        let baseline = self.observe_under(first);
        for optimization in permutations {
            assert_eq!(
                self.observe_under(optimization),
                baseline,
                "{:?}: the run under {optimization:?} differs from under {first:?}",
                self.source
            );
        }
        baseline
    }

    /// The tail value of a run that must complete.
    pub(crate) fn run(&self) -> Value {
        self.finish().tail
    }

    /// The tail value of a run under exactly `optimization`, which must complete.
    /// Only for a script whose result may rightly depend on the options, such as
    /// one probing whether a value is shared; others use [`run`](Self::run).
    pub(crate) fn run_under(&self, optimization: OptimizationOptions) -> Value {
        match self.observe_under(optimization).outcome {
            Ok(finished) => finished.tail,
            Err(message) => panic!(
                "{:?} should run under {optimization:?}, but raised: {message}",
                self.source
            ),
        }
    }

    /// A run that must complete.
    pub(crate) fn finish(&self) -> Finished {
        self.outcome()
            .unwrap_or_else(|message| panic!("{:?} should run, but raised: {message}", self.source))
    }

    /// The error message of a run that must raise.
    pub(crate) fn raises(&self) -> String {
        match self.outcome() {
            Ok(finished) => panic!(
                "{:?} should raise, but produced {:?}",
                self.source, finished.tail
            ),
            Err(message) => message,
        }
    }

    /// The diagnostics of a script that must not compile. Every optimization
    /// permutation must reject it with the same diagnostics.
    pub(crate) fn compile_errors(&self) -> CompilerErrors {
        let mut permutations = optimization_permutations();
        let first = permutations
            .next()
            .expect("there is always one permutation");
        let baseline = self.compile_error_under(first);
        for optimization in permutations {
            assert_eq!(
                self.compile_error_under(optimization).render_plain(),
                baseline.render_plain(),
                "{:?}: the diagnostics under {optimization:?} differ from under {first:?}",
                self.source
            );
        }
        baseline
    }

    /// The top-level function's code under exactly `optimization`.
    ///
    /// A code-shape test pins the options it is about and leaves the rest off,
    /// e.g. `UNOPTIMIZED.with(Optimization::BranchEliminate, true)`, so no
    /// other optimization, present or future, changes the code it inspects.
    pub(crate) fn code(&self, optimization: OptimizationOptions) -> Emitted {
        Emitted::new(self.function_under(optimization), optimization)
    }

    /// The top-level function's code under each optimization permutation that
    /// `select` accepts.
    pub(crate) fn code_where(&self, select: impl Fn(&OptimizationOptions) -> bool) -> Vec<Emitted> {
        let emitted: Vec<Emitted> = optimization_permutations()
            .filter(select)
            .map(|optimization| Emitted::new(self.function_under(optimization), optimization))
            .collect();
        assert!(
            !emitted.is_empty(),
            "the selection accepts no optimization permutation"
        );
        emitted
    }

    fn function_under(&self, optimization: OptimizationOptions) -> Arc<CompiledFunction> {
        let output = self.compile(optimization).unwrap_or_else(|errors| {
            panic!(
                "{:?} should compile under {optimization:?}:\n{}",
                self.source,
                errors.render_plain()
            )
        });
        let closure = output
            .code
            .close(self.captures.clone())
            .expect("every capture the script uses is supplied");
        Arc::new(closure.inner_fn().clone())
    }

    fn compile(&self, optimization: OptimizationOptions) -> Result<CompilerOutput, CompilerErrors> {
        let options = CompilerOptions {
            optimization_options: optimization,
            implicit_export: self.implicit_export,
        };
        let scope: Vec<&str> = self.scope.iter().map(String::as_str).collect();
        compile_in_scope(&self.filename, &self.source, options, &scope)
    }

    fn observe_under(&self, optimization: OptimizationOptions) -> Observed {
        let output = self.compile(optimization).unwrap_or_else(|errors| {
            panic!(
                "{:?} should compile under {optimization:?}:\n{}",
                self.source,
                errors.render_plain()
            )
        });
        let closure = output
            .code
            .close(self.captures.clone())
            .expect("every capture the script uses is supplied");
        let printed = Arc::new(Mutex::new(Vec::new()));
        let sink = {
            let printed = Arc::clone(&printed);
            move |text: &str| printed.lock().unwrap().push(text.to_string())
        };
        let config = VmRuntimeConfiguration {
            max_call_depth: self.max_call_depth,
            print_sink: Arc::new(sink),
            ..Default::default()
        };
        let outcome = Vm::factory()
            .configuration(config)
            .with_importer(Arc::clone(&self.importer))
            .build(closure)
            .expect("closure builds")
            .run()
            .map(|result| Finished {
                tail: result.tail().clone(),
                exports: result
                    .exports()
                    .map(|(name, value)| (name.to_string(), value.clone()))
                    .collect(),
            })
            .map_err(|error| error.into_error().message().into_owned());
        let printed = printed.lock().unwrap().clone();
        Observed { outcome, printed }
    }

    fn compile_error_under(&self, optimization: OptimizationOptions) -> CompilerErrors {
        match self.compile(optimization) {
            Ok(_) => panic!(
                "{:?} should not compile, but did under {optimization:?}",
                self.source
            ),
            Err(errors) => errors,
        }
    }
}

/// [`Script::run`] for a script with no enclosing scope.
pub(crate) fn run(source: &str) -> Value {
    Script::new(source).run()
}

/// [`Script::raises`] for a script with no enclosing scope.
pub(crate) fn raises(source: &str) -> String {
    Script::new(source).raises()
}

/// [`Script::compile_errors`] for a script with no enclosing scope.
pub(crate) fn compile_errors(source: &str) -> CompilerErrors {
    Script::new(source).compile_errors()
}

/// A compiled function's code under one optimization permutation. Its `Debug`
/// form names the permutation, for assertion messages.
#[derive(Debug)]
pub(crate) struct Emitted {
    pub(crate) optimization: OptimizationOptions,
    pub(crate) code: Vec<Bytecode>,
    function: Arc<CompiledFunction>,
}

impl Emitted {
    fn new(function: Arc<CompiledFunction>, optimization: OptimizationOptions) -> Self {
        Self {
            optimization,
            code: function.code.clone(),
            function,
        }
    }

    /// The function nested at `index` in this one (the `index`-th closure it
    /// creates, in code order), such as a lambda's body.
    pub(crate) fn nested(&self, index: usize) -> Emitted {
        let child = self.function.child_fns.get(index).unwrap_or_else(|| {
            panic!(
                "there is no nested function {index}; the function has {}",
                self.function.child_fns.len()
            )
        });
        Emitted::new(Arc::clone(child), self.optimization)
    }

    /// The function's name.
    pub(crate) fn name(&self) -> &str {
        &self.function.name
    }

    /// The local slot holding the binding `name`.
    pub(crate) fn slot_named(&self, name: &str) -> usize {
        self.function
            .name_table
            .iter()
            .position(|entry| entry.name == name)
            .unwrap_or_else(|| panic!("the function has no local `{name}`"))
    }

    /// How many captured values a closure over this function is created with.
    pub(crate) fn num_captures(&self) -> usize {
        self.function.num_captures
    }

    /// The function's constant pool, which `LoadConst` indexes.
    pub(crate) fn constants(&self) -> &[Value] {
        &self.function.constants
    }

    /// The Map keys the function's constant-key ops look up.
    pub(crate) fn key_constants(&self) -> &[MapKey] {
        &self.function.key_constants
    }

    /// How many times `op` appears in the code.
    pub(crate) fn count(&self, op: &Bytecode) -> usize {
        self.code
            .iter()
            .filter(|candidate| *candidate == op)
            .count()
    }
}
