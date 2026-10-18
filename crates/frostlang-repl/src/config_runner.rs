//! [`ConfigRunner`]: how the [config file](crate::Repl#config-file), and the
//! functions it holds, run. Apart from any session, under small limits, and
//! importing only `std.string`, `std.math`, and the `repl` component.

use std::num::NonZeroU64;
use std::sync::Arc;

#[cfg(feature = "line-editor")]
use frostlang::compile::compile_in_scope;
use frostlang::compile::{CompilerOptions, Diagnostics, compile_program};
use frostlang::{
    Closure, FrostError, HostComponent, IdleVm, Importer, ImporterBuilder, ProgramParts, Stdlib,
    Value, Vm, VmRuntimeConfiguration, frost_closure, stdlib,
};

#[cfg(test)]
mod tests;

/// What a script, or a function it holds, may spend in one run. A frontend
/// may call a function before every segment, so one that runs away must fail
/// fast.
fn limits() -> VmRuntimeConfiguration {
    const FUEL: NonZeroU64 = NonZeroU64::new(100_000).unwrap();
    const CALL_DEPTH: NonZeroU64 = NonZeroU64::new(256).unwrap();
    const IMPORT_DEPTH: NonZeroU64 = NonZeroU64::new(4).unwrap();
    VmRuntimeConfiguration::default()
        .with_fuel(Some(FUEL))
        .with_max_call_depth(Some(CALL_DEPTH))
        .with_max_import_depth(Some(IMPORT_DEPTH))
}

/// Why a script did not run to its end.
#[derive(Debug)]
pub(crate) enum ScriptFailure {
    Compile(Diagnostics),
    Run(FrostError),
}

/// Runs a configuration script, and the functions it holds, on a Vm kept warm
/// between runs.
pub(crate) struct ConfigRunner {
    importer: Arc<Importer>,
    idle_vm: Option<IdleVm>,
}

impl ConfigRunner {
    /// A runner whose `repl.colors` and `repl.style` color their text only if
    /// `color` is true.
    pub(crate) fn new(color: bool) -> Self {
        let stdlib = Stdlib::new()
            .with_module(stdlib::string())
            .and_then(|stdlib| stdlib.with_module(stdlib::math()))
            .expect("the modules are distinct");
        let importer = ImporterBuilder::new()
            .with_stdlib(stdlib)
            .with_component(component(color))
            .expect("`repl` is free to claim")
            .build();
        Self {
            importer,
            idle_vm: None,
        }
    }

    /// The exports of the script `source`, from the file `filename`.
    pub(crate) fn run_script(
        &mut self,
        filename: &str,
        source: &str,
    ) -> Result<Vec<(String, Value)>, ScriptFailure> {
        // Lints in a config file are not shown, so are not found.
        let options = CompilerOptions::new().with_lints(false);
        let program = compile_program(filename, source, options)
            .map_err(ScriptFailure::Compile)?
            .code;
        let closure = program
            .into_closure()
            .expect("a whole script captures nothing");
        self.run(closure, None)
            .map(|ran| ran.exports)
            .map_err(ScriptFailure::Run)
    }

    /// `function`, a native, as a closure, so that it runs as a script does.
    #[cfg(feature = "line-editor")]
    pub(crate) fn wrap(&mut self, function: Value) -> Result<Arc<Closure>, FrostError> {
        const SOURCE: &str = "fn argument -> function(argument)";
        let program = compile_in_scope("<wrapper>", SOURCE, CompilerOptions::new(), &["function"])
            .expect("the wrapper compiles")
            .code;
        let closure = program
            .close([("function", function)])
            .expect("the wrapper captures only `function`");
        match self.run(closure, None)?.tail {
            Value::Closure(wrapper) => Ok(wrapper),
            other => unreachable!("the wrapper is a lambda, not {}", other.type_name()),
        }
    }

    /// What `closure` returns, given `argument`.
    #[cfg(feature = "line-editor")]
    pub(crate) fn call(
        &mut self,
        closure: Arc<Closure>,
        argument: Value,
    ) -> Result<Value, FrostError> {
        self.run(closure, Some(argument)).map(|ran| ran.tail)
    }

    /// The tail and exports of `closure`, given `argument`, if any.
    fn run(&mut self, closure: Arc<Closure>, argument: Option<Value>) -> Result<Ran, FrostError> {
        let vm = match self.idle_vm.take() {
            Some(idle_vm) => idle_vm.build(closure),
            None => Vm::factory()
                .configuration(limits())
                .with_importer(Arc::clone(&self.importer))
                .build(closure),
        };
        match vm.run_with_args(argument) {
            Ok(result) => {
                let ProgramParts {
                    tail,
                    exports,
                    idle_vm,
                } = result.into_parts();
                self.idle_vm = Some(idle_vm);
                Ok(Ran { tail, exports })
            }
            Err(failure) => {
                let (error, idle_vm) = failure.into_parts();
                self.idle_vm = Some(idle_vm);
                Err(error)
            }
        }
    }
}

/// What a run gave back.
struct Ran {
    // Read only by a frontend's calls.
    #[cfg_attr(not(feature = "line-editor"), expect(dead_code))]
    tail: Value,
    exports: Vec<(String, Value)>,
}

/// The `repl` component: `repl.colors` and `repl.style` hold functions that
/// wrap a String in ANSI codes, or, if `color` is false, return it unchanged.
fn component(color: bool) -> HostComponent {
    HostComponent::builder("repl")
        .module("colors", |colors| {
            COLORS
                .into_iter()
                .fold(colors, |colors, (key, name, code)| {
                    colors.value(key, painter(name, color, code, FOREGROUND_RESET))
                })
        })
        .module("style", |style| {
            STYLES
                .into_iter()
                .fold(style, |style, (key, name, on, off)| {
                    style.value(key, painter(name, color, on, off))
                })
        })
        .build()
}

/// The SGR code that returns the foreground to the terminal's default.
const FOREGROUND_RESET: u8 = 39;

/// Each of `repl.colors`: its key, its function's name, and its SGR code.
const COLORS: [(&str, &str, u8); 16] = [
    ("black", "repl.colors.black", 30),
    ("red", "repl.colors.red", 31),
    ("green", "repl.colors.green", 32),
    ("yellow", "repl.colors.yellow", 33),
    ("blue", "repl.colors.blue", 34),
    ("magenta", "repl.colors.magenta", 35),
    ("cyan", "repl.colors.cyan", 36),
    ("white", "repl.colors.white", 37),
    ("bright_black", "repl.colors.bright_black", 90),
    ("bright_red", "repl.colors.bright_red", 91),
    ("bright_green", "repl.colors.bright_green", 92),
    ("bright_yellow", "repl.colors.bright_yellow", 93),
    ("bright_blue", "repl.colors.bright_blue", 94),
    ("bright_magenta", "repl.colors.bright_magenta", 95),
    ("bright_cyan", "repl.colors.bright_cyan", 96),
    ("bright_white", "repl.colors.bright_white", 97),
];

/// Each of `repl.style`: its key, its function's name, and the SGR codes that
/// turn it on and off.
const STYLES: [(&str, &str, u8, u8); 4] = [
    ("bold", "repl.style.bold", 1, 22),
    ("dim", "repl.style.dim", 2, 22),
    ("italic", "repl.style.italic", 3, 23),
    ("underline", "repl.style.underline", 4, 24),
];

/// A function named `name` that wraps a String between the SGR codes `on` and
/// `off`, or, if `color` is false, returns it unchanged. Text painted within
/// it keeps its paint after an inner painter's text ends.
fn painter(name: &'static str, color: bool, on: u8, off: u8) -> Value {
    frost_closure!(name, move |text: String| {
        if color {
            let (on, off) = (format!("\x1b[{on}m"), format!("\x1b[{off}m"));
            // An inner painter's `off` ends this painter's `on` too, as when
            // a color ends inside another color, or bold ends inside dim.
            let text = text.replace(&off, &format!("{off}{on}"));
            format!("{on}{text}{off}")
        } else {
            text
        }
    })
}
