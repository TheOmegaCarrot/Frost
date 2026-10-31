//! A tail call that keeps the calling frame while the callee runs (one to a
//! native, from any frame, or any one from the bottom frame) releases the
//! frame's locals first, as an elided frame's are. The top-level function's
//! exported slots stay, since the program's result reads them.
//!
//! The release tests run hand-written bytecode, pinning the release at the
//! opcode whatever the compiler emits around a tail call. Each one holds a second
//! reference to an Array in a local the code never reads again, then hands the
//! Array to the `unshared` probe through a tail call: the probe sees the only
//! reference only if that local was released before it ran. A compiled script's
//! exports are checked too, through `Script`.

use crate::common;
use crate::script::{self, Script};
use crate::unshared::unshared;

use std::collections::BTreeMap;
use std::sync::Arc;

use common::{Pop, entry, func, func_with_captures};
use frostlang::bytecode::{Bytecode, CompiledFunction, NameEntry};
use frostlang::{Arity, ProgramResult, Value, Vm};

use Bytecode::*;

// The top-level slots: the `unshared` capture, then `xs`, `held`, and the
// exported `kept`.
const UNSHARED: u64 = 0;
const XS: u64 = 1;
const HELD: u64 = 2;
const KEPT: u64 = 3;

fn top_level_names() -> Vec<NameEntry> {
    vec![
        entry("unshared", false),
        entry("xs", false),
        entry("held", false),
        entry("kept", true),
    ]
}

/// Define `kept` as `[3]`.
fn define_kept() -> Vec<Bytecode> {
    vec![PushInt(3), MakeArray(1), DefLocal(KEPT)]
}

/// Define `xs` as a new `[1, 2]`, `held` as a second reference to it, and
/// `kept`.
fn define_xs_held_and_kept() -> Vec<Bytecode> {
    [
        vec![
            PushInt(1),
            PushInt(2),
            MakeArray(2),
            DefLocal(XS),
            LoadLocal(XS),
            DefLocal(HELD),
        ],
        define_kept(),
    ]
    .concat()
}

/// Run `code` as the top level, splicing in its leading `Pop`, with the slots of
/// [`top_level_names`] and `unshared` captured into the first.
fn run_top_level(code: Vec<Bytecode>, child_fns: Vec<Arc<CompiledFunction>>) -> ProgramResult {
    let top = func_with_captures(
        [vec![Pop], code].concat(),
        Arity::Exact(0),
        top_level_names(),
        child_fns,
        1,
    );
    let closure = top
        .assert_well_formed()
        .close(BTreeMap::from([("unshared".to_string(), unshared())]))
        .expect("`unshared` is supplied");
    Vm::factory()
        .build(closure)
        .run()
        .expect("the program runs")
}

/// Assert `result` holds `expected_tail`, and its export `kept` is still `[3]`.
fn assert_released_and_kept(result: &ProgramResult, expected_tail: Value, case: &str) {
    assert_eq!(
        result.tail(),
        &expected_tail,
        "{case}: the callee must not see `held`'s reference"
    );
    assert_eq!(
        result.get_export("kept"),
        Some(&Value::from(vec![Value::Int(3)])),
        "{case}: the export stays"
    );
}

#[test]
fn a_tail_call_to_a_native_from_the_bottom_frame_releases_its_locals() {
    let code = [
        define_xs_held_and_kept(),
        vec![LoadLocal(UNSHARED), ConsumeLocal(XS), TailCall(1)],
    ]
    .concat();
    let result = run_top_level(code, Vec::new());
    assert_released_and_kept(&result, Value::Bool(true), "a native callee");
}

#[test]
fn a_spread_tail_call_to_a_native_from_the_bottom_frame_releases_its_locals() {
    let code = [
        define_xs_held_and_kept(),
        vec![
            LoadLocal(UNSHARED),
            ConsumeLocal(XS),
            MakeArray(1),
            DynTailCall,
        ],
    ]
    .concat();
    let result = run_top_level(code, Vec::new());
    assert_released_and_kept(&result, Value::Bool(true), "a native callee, spread");
}

#[test]
fn a_tail_call_to_a_closure_from_the_bottom_frame_releases_its_locals() {
    // `fn probe, v -> [probe(v)]`: the probe runs in the callee's own frame,
    // above the kept bottom frame.
    let callee = func(
        vec![
            DefLocal(1),
            DefLocal(0),
            Pop,
            ConsumeLocal(0),
            ConsumeLocal(1),
            Call(1),
            MakeArray(1),
        ],
        Arity::Exact(2),
        vec![entry("probe", false), entry("v", false)],
        Vec::new(),
    );
    let code = [
        define_xs_held_and_kept(),
        vec![
            CreateClosure {
                child: 0,
                captures: 0,
            },
            LoadLocal(UNSHARED),
            ConsumeLocal(XS),
            TailCall(2),
        ],
    ]
    .concat();
    let result = run_top_level(code, vec![callee]);
    assert_released_and_kept(
        &result,
        Value::from(vec![Value::Bool(true)]),
        "a closure callee",
    );
}

#[test]
fn a_tail_call_to_a_native_from_a_called_frame_releases_its_locals() {
    // `fn probe, xs -> { def held = xs; probe(xs) }`, called from the top level,
    // whose own `xs` and `held` go unused.
    let caller = func(
        vec![
            DefLocal(1),
            DefLocal(0),
            Pop,
            LoadLocal(1),
            DefLocal(2),
            ConsumeLocal(0),
            ConsumeLocal(1),
            TailCall(1),
        ],
        Arity::Exact(2),
        vec![
            entry("probe", false),
            entry("xs", false),
            entry("held", false),
        ],
        Vec::new(),
    );
    let code = [
        define_kept(),
        vec![
            CreateClosure {
                child: 0,
                captures: 0,
            },
            LoadLocal(UNSHARED),
            PushInt(1),
            PushInt(2),
            MakeArray(2),
            Call(2),
        ],
    ]
    .concat();
    let result = run_top_level(code, vec![caller]);
    assert_released_and_kept(
        &result,
        Value::Bool(true),
        "a native callee, from a called frame",
    );
}

#[test]
fn a_scripts_exports_survive_its_tail_call() {
    // A native callee, then a closure one.
    for tail in ["len(other)", "count(other)"] {
        let source = format!(
            r"
            export def kept = [1, 2]
            def other = [kept]
            defn count(x) -> len(x)
            {tail}
            "
        );
        let finished = Script::new(&source).finish();
        assert_eq!(finished.tail, Value::Int(1), "{source}");
        assert_eq!(
            finished.exports,
            BTreeMap::from([("kept".to_string(), script::run("[1, 2]"))]),
            "{source}"
        );
    }
}

#[test]
fn implicitly_exported_bindings_survive_a_tail_call() {
    // Each top-level binding is exported, as the REPL compiles a script.
    let source = r"
        def a = [1, 2]
        def b = [a]
        len(b)
        ";
    let finished = Script::new(source).implicit_export().finish();
    assert_eq!(finished.tail, Value::Int(1), "{source}");
    assert_eq!(
        finished.exports,
        BTreeMap::from([
            ("a".to_string(), script::run("[1, 2]")),
            ("b".to_string(), script::run("[[1, 2]]")),
        ]),
        "{source}"
    );
}
