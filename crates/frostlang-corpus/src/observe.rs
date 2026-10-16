use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use frostlang::compile::{CompilerOptions, compile_in_scope};
use frostlang::{Value, Vm, VmRuntimeConfiguration};

/// What compiling and running a script does under one set of options.
#[derive(Debug, PartialEq)]
pub(crate) enum Observed {
    /// Compilation failed, with these diagnostics, narrated.
    Rejected(String),
    /// Compilation succeeded, and the run produced this.
    Ran {
        /// The completed run, or the message of the error it raised.
        outcome: Result<Finished, String>,
        /// The text of each `print`, whether or not the run completed.
        printed: Vec<String>,
    },
}

/// A completed run.
#[derive(Debug, PartialEq)]
pub(crate) struct Finished {
    pub(crate) result: Value,
    pub(crate) exports: BTreeMap<String, Value>,
}

/// Compile `source` under `options`, with each of `inputs` in its enclosing
/// scope, and run it if it compiles, supplying their values.
pub(crate) fn observe(
    filename: &str,
    source: &str,
    inputs: &BTreeMap<String, Value>,
    options: CompilerOptions,
) -> Observed {
    let scope: Vec<&str> = inputs.keys().map(String::as_str).collect();
    let output = match compile_in_scope(filename, source, options, &scope) {
        Ok(output) => output,
        Err(diagnostics) => return Observed::Rejected(diagnostics.render_narrated()),
    };
    let closure = output
        .code
        .close(inputs.clone())
        .expect("every name in scope is supplied");
    let printed = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let printed = Arc::clone(&printed);
        move |text: &str| printed.lock().unwrap().push(text.to_string())
    };
    let configuration = VmRuntimeConfiguration::default().with_print_sink(Arc::new(sink));
    let outcome = Vm::factory()
        .configuration(configuration)
        .build(closure)
        .run()
        .map(|result| Finished {
            result: result.tail().clone(),
            exports: result
                .exports()
                .map(|(name, value)| (name.to_string(), value.clone()))
                .collect(),
        })
        .map_err(|error| error.into_error().message().into_owned());
    let printed = printed.lock().unwrap().clone();
    Observed::Ran { outcome, printed }
}
