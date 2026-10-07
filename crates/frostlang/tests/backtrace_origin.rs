//! Backtrace frames carry the file each function was compiled from, and
//! `with_backtrace` names those files when the frames come from more than one.

use std::collections::BTreeMap;

use frostlang::compile::{CompilerOptions, OptimizationOptions, compile_in_scope};
use frostlang::{BacktraceFrame, FrostError, RunError, Value, Vm};

const OPTIONS: CompilerOptions =
    CompilerOptions::new().with_optimization(OptimizationOptions::NONE);

/// Run `source`, compiled as `filename` with `captures` in scope.
fn run(
    filename: &str,
    source: &str,
    captures: BTreeMap<String, Value>,
) -> Result<BTreeMap<String, Value>, FrostError> {
    let scope: Vec<&str> = captures.keys().map(String::as_str).collect();
    let closure = compile_in_scope(filename, source, OPTIONS, &scope)
        .unwrap_or_else(|errors| panic!("{filename} compiles:\n{}", errors.render_plain()))
        .code
        .close(captures)
        .expect("every capture is supplied");
    let result = Vm::factory()
        .build(closure)?
        .run()
        .map_err(RunError::into_error)?;
    Ok(result
        .exports()
        .map(|(name, value)| (name.to_string(), value.clone()))
        .collect())
}

/// The error from `main.frst` calling `boom` from `lib.frst`, which raises.
fn error_across_files() -> FrostError {
    let lib = run(
        "lib/lib.frst",
        r"
        export defn boom() -> [error('boom')]
        ",
        BTreeMap::new(),
    )
    .expect("the library runs");
    let lib = Value::Map(lib.into_iter().map(|(k, v)| (k.into(), v)).collect());
    run(
        "main.frst",
        r"
        defn go() -> [lib.boom()]
        go()
        ",
        BTreeMap::from([("lib".to_string(), lib)]),
    )
    .expect_err("main raises")
}

/// Each frame as `name@origin`, `-` for no origin.
fn frames(err: &FrostError) -> Vec<String> {
    err.backtrace()
        .iter()
        .map(|frame| format!("{}@{}", frame.name(), frame.origin().unwrap_or("-")))
        .collect()
}

#[test]
fn each_frame_carries_its_function_origin() {
    let err = error_across_files();
    assert_eq!(
        frames(&err),
        [
            "error@-",
            "boom@lib.frst",
            "go@main.frst",
            "<main>@main.frst"
        ],
        "a native has no origin; each compiled function has its file's base name"
    );
}

#[test]
fn with_backtrace_names_the_files_when_frames_span_several() {
    let err = error_across_files();
    assert_eq!(
        err.with_backtrace().to_string(),
        "boom\n  \
         in error\n  \
         in boom (lib.frst)\n  \
         in go (main.frst)\n  \
         in <main> (main.frst)"
    );
}

#[test]
fn with_backtrace_omits_the_file_when_every_frame_shares_it() {
    let err = run(
        "main.frst",
        r"
        defn inner() -> [error('boom')]
        inner()
        ",
        BTreeMap::new(),
    )
    .expect_err("main raises");
    assert_eq!(
        err.with_backtrace().to_string(),
        "boom\n  in error\n  in inner\n  in <main>"
    );
}

#[test]
fn a_frame_displays_as_its_name() {
    let err = error_across_files();
    let shown: Vec<String> = err
        .backtrace()
        .iter()
        .map(BacktraceFrame::to_string)
        .collect();
    assert_eq!(shown, ["error", "boom", "go", "<main>"]);
}
