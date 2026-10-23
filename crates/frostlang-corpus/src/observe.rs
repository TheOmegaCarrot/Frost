use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use frostlang::compile::{CompilerOptions, Diagnostic, compile_program};
use frostlang::{HostComponent, ImporterBuilder, MapKey, Value, Vm, VmRuntimeConfiguration};

/// What compiling and running a script does under one set of options.
#[derive(Debug, PartialEq)]
pub(crate) enum Observed {
    /// Compilation failed, with these diagnostics, narrated.
    Rejected(String),
    /// Compilation succeeded, and the run produced this.
    Ran {
        /// Each lint, narrated, in source order.
        lints: Vec<String>,
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

/// Compile `source` on its own under `options`, and run it if it compiles,
/// with `inputs` as the content of a host component named `inputs`, which the
/// script imports. An import is a runtime call, so the compiler knows nothing
/// of an input's value.
pub(crate) fn observe(
    filename: &str,
    source: &str,
    inputs: &BTreeMap<String, Value>,
    options: CompilerOptions,
) -> Observed {
    let output = match compile_program(filename, source, options) {
        Ok(output) => output,
        Err(diagnostics) => return Observed::Rejected(diagnostics.render_narrated()),
    };
    let lints = output
        .lints
        .iter()
        .map(Diagnostic::render_narrated)
        .collect();
    let closure = output
        .code
        .into_closure()
        .expect("a script compiled on its own captures nothing");
    let inputs: Value = inputs
        .iter()
        .map(|(name, value)| (MapKey::from(name.as_str()), value.clone()))
        .collect();
    let component = HostComponent::new("inputs", inputs).expect("`inputs` is a valid name");
    let importer = ImporterBuilder::new()
        .with_component(component)
        .expect("the importer has no other component")
        .build();
    let printed = Arc::new(Mutex::new(Vec::new()));
    let sink = {
        let printed = Arc::clone(&printed);
        move |text: &str| printed.lock().unwrap().push(text.to_string())
    };
    let configuration = VmRuntimeConfiguration::default().with_print_sink(Arc::new(sink));
    let outcome = Vm::factory()
        .configuration(configuration)
        .with_importer(importer)
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
    Observed::Ran {
        lints,
        outcome,
        printed,
    }
}
