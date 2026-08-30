//! Error-flow tests for the VM: raising, unwinding, backtrace accumulation, and `try_call` (Frost's catch primitive).
//!
//! The model under test:
//!   * Opcodes and native calls `?`-propagate errors straight out of the current `execute_function` activation.
//!   * The boundary that entered the activation cleans up: `Vm::run` at the top level (no `NativeFrame`: the error reaches the host),
//!     or `NativeCtx::invoke` at a native re-entry (frames + operand stack are truncated back to the entry floor before Err is returned).
//!   * The backtrace is accumulated as abandoned frames are discarded (`unwind_frames`), plus a native's own name in `run_native`.
//!   * `try_call` is the one native that declines to `?`: it catches and reifies the outcome into a result map.
//!
//! Error sources used here: division/modulus by zero, an `add` type error,
//! calling a non-function, an arity mismatch, `ProduceError`, plus a couple of
//! native fixtures (`boom`, `apply`).

mod common;

use std::sync::Arc;

use common::{entry, global_slot};
use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FormatVersion, FrostError, FrostMap, FrostResult, NameEntry,
    NativeCtx, NativeFunction, ProgramResult, Value, Vm,
};

// ============================================================
// Helpers
// ============================================================

/// A compiled function with an explicit `name`, so backtraces are checkable.
fn named(
    name: &str,
    code: Vec<Bytecode>,
    arity: Arity,
    names: Vec<NameEntry>,
    children: Vec<Arc<CompiledFunction>>,
) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        version: FormatVersion,
        name: name.to_string(),
        code,
        child_fns: children,
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: names,
        num_captures: 0,
        arity,
    })
}

/// A native function `Value`.
fn native(
    name: &'static str,
    arity: Arity,
    f: impl Fn(NativeCtx<'_>, &mut [Value]) -> FrostResult + Send + Sync + 'static,
) -> Value {
    Value::NativeFunction(Arc::new(NativeFunction::new(name, arity, f)))
}

/// `CreateClosure` for a capture-less child function at `idx`.
fn closure(idx: u32) -> Bytecode {
    Bytecode::CreateClosure(idx as usize)
}

/// Slot index of the `try_call` global (the VM is built with the default set).
fn try_call_slot() -> usize {
    global_slot("try_call")
}

/// Run a program to completion, surfacing the result (Ok or Err).
fn run(program: Arc<CompiledFunction>) -> Result<ProgramResult, FrostError> {
    run_with(program, Vec::new())
}

/// Run `program` with `bindings` supplied as captures (slot `i` is the i-th
/// binding; the program's `name_table` must list them first, in order).
/// Splices in the leading fn-value `Pop` the top-level needs.
fn run_with(
    program: Arc<CompiledFunction>,
    bindings: Vec<(&str, Value)>,
) -> Result<ProgramResult, FrostError> {
    let top = CompiledFunction {
        code: std::iter::once(Bytecode::Pop)
            .chain(program.code.iter().copied())
            .collect(),
        num_captures: bindings.len(),
        ..(*program).clone()
    };
    let captures: std::collections::BTreeMap<String, Value> = bindings
        .into_iter()
        .map(|(n, v)| (n.to_string(), v))
        .collect();
    let closure = Arc::new(top)
        .assert_trusted()
        .close(captures)
        .expect("all captures provided");
    Vm::factory()
        .build(closure)
        .unwrap()
        .run()
        .map_err(|e| e.into_error())
}

/// `apply(f, ...rest)`: a re-entrant native that invokes `f` with the rest of
/// its args and *propagates* any error (it does not catch). Used to exercise an
/// error flowing through a native frame.
fn apply_native() -> Value {
    native("apply", Arity::AtLeast(1), |mut ctx, args| {
        let f = args[0].clone();
        ctx.invoke(
            &f,
            args[1..]
                .iter_mut()
                .map(|v| std::mem::replace(v, Value::Null)),
        )
    })
}

/// A native that always raises.
fn boom_native() -> Value {
    native("boom", Arity::Exact(0), |_ctx, _args| {
        Err(FrostError::from_static("boom!"))
    })
}

/// Closure body that raises via division by zero: `fn -> 1 / 0`.
fn div_zero_body() -> Vec<Bytecode> {
    vec![
        Bytecode::Pop, // pop the closure's own function value
        Bytecode::PushInt(1),
        Bytecode::PushInt(0),
        Bytecode::Divide,
    ]
}

fn expect_map(v: &Value) -> &FrostMap {
    v.as_map().expect("expected a Map result")
}

/// Extract the `trace` array of a failure result map as a `Vec<String>`.
fn trace_of(map: &FrostMap) -> Vec<String> {
    map.get_str("trace")
        .expect("failure map has a `trace`")
        .as_array()
        .expect("`trace` is an Array")
        .iter()
        .map(|v| v.as_str().expect("trace entry is a String").to_string())
        .collect()
}

/// A backtrace as `&str`s, for ergonomic comparison.
fn backtrace(err: &FrostError) -> Vec<&str> {
    err.backtrace().iter().map(String::as_str).collect()
}

// ============================================================
// Top-level errors: no NativeFrame, error reaches the host via run()
// ============================================================

#[test]
fn top_level_division_by_zero() {
    let program = named(
        "main",
        vec![Bytecode::PushInt(1), Bytecode::PushInt(0), Bytecode::Divide],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let err = run(program).unwrap_err();
    assert_eq!(err.message(), "Division by zero");
    assert_eq!(backtrace(&err), vec!["main"]);
}

#[test]
fn top_level_modulus_by_zero() {
    let program = named(
        "main",
        vec![
            Bytecode::PushInt(7),
            Bytecode::PushInt(0),
            Bytecode::Modulus,
        ],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let err = run(program).unwrap_err();
    assert_eq!(err.message(), "Modulus by zero");
}

#[test]
fn top_level_type_error() {
    // 1 + null: incompatible operand types.
    let program = named(
        "main",
        vec![Bytecode::PushInt(1), Bytecode::PushNull, Bytecode::Add],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let err = run(program).unwrap_err();
    assert!(
        err.message().contains("incompatible types"),
        "unexpected message: {}",
        err.message()
    );
}

#[test]
fn top_level_call_non_function() {
    // Call an Int.
    let program = named(
        "main",
        vec![Bytecode::PushInt(5), Bytecode::Call(0)],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let err = run(program).unwrap_err();
    assert!(
        err.message().contains("non-function"),
        "unexpected message: {}",
        err.message()
    );
}

#[test]
fn top_level_arity_mismatch() {
    // identity wants 1 arg; call it with 0.
    let identity = named(
        "identity",
        vec![Bytecode::DefLocal(0), Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::Exact(1),
        vec![entry("x", false)],
        vec![],
    );
    let program = named(
        "main",
        vec![closure(0), Bytecode::Call(0)],
        Arity::Exact(0),
        vec![],
        vec![identity],
    );
    let err = run(program).unwrap_err();
    assert!(
        err.message().contains("expects 1"),
        "unexpected message: {}",
        err.message()
    );
}

// ============================================================
// Backtrace accumulation across Frost frames (terminal path)
// ============================================================

#[test]
fn backtrace_records_each_calling_frame_innermost_first() {
    // main -> outer -> inner, inner divides by zero. All non-tail calls, so the
    // three frames coexist in one activation and the walk records them top-down.
    let inner = named("inner", div_zero_body(), Arity::Exact(0), vec![], vec![]);
    let outer = named(
        "outer",
        vec![Bytecode::Pop, closure(0), Bytecode::Call(0)],
        Arity::Exact(0),
        vec![],
        vec![inner],
    );
    let program = named(
        "main",
        vec![closure(0), Bytecode::Call(0)],
        Arity::Exact(0),
        vec![],
        vec![outer],
    );
    let err = run(program).unwrap_err();
    assert_eq!(backtrace(&err), vec!["inner", "outer", "main"]);
}

#[test]
fn backtrace_includes_native_name_at_top_level() {
    // main calls a native that raises; the native contributes its own frame name.
    let program = named(
        "main",
        vec![Bytecode::LoadLocal(0), Bytecode::Call(0)],
        Arity::Exact(0),
        vec![entry("boom", false)],
        vec![],
    );
    let err = run_with(program, vec![("boom", boom_native())]).unwrap_err();
    assert_eq!(err.message(), "boom!");
    assert_eq!(backtrace(&err), vec!["boom", "main"]);
}

// ============================================================
// try_call: success
// ============================================================

#[test]
fn try_call_success_wraps_value() {
    let constant = named(
        "constant",
        vec![Bytecode::Pop, Bytecode::PushInt(42)],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1),
        ],
        Arity::Exact(0),
        vec![],
        vec![constant],
    );
    let result = run(program).unwrap();
    let map = expect_map(result.tail());
    assert_eq!(map.get_str("ok"), Some(&Value::Bool(true)));
    assert_eq!(map.get_str("value"), Some(&Value::Int(42)));
    // Exactly { ok, value }: no error/trace on success.
    assert_eq!(map.len(), 2);
    assert!(map.get_str("error").is_none());
    assert!(map.get_str("trace").is_none());
}

#[test]
fn try_call_forwards_arguments() {
    // try_call(add, [2, 3]) -> { ok: true, value: 5 }: the args array is spread.
    let add = named(
        "add",
        vec![
            Bytecode::DefLocal(1),
            Bytecode::DefLocal(0),
            Bytecode::Pop,
            Bytecode::LoadLocal(0),
            Bytecode::LoadLocal(1),
            Bytecode::Add,
        ],
        Arity::Exact(2),
        vec![entry("a", false), entry("b", false)],
        vec![],
    );
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::PushInt(2),
            Bytecode::PushInt(3),
            Bytecode::MakeArray(2),
            Bytecode::Call(2),
        ],
        Arity::Exact(0),
        vec![],
        vec![add],
    );
    let result = run(program).unwrap();
    let map = expect_map(result.tail());
    assert_eq!(map.get_str("ok"), Some(&Value::Bool(true)));
    assert_eq!(map.get_str("value"), Some(&Value::Int(5)));
}

#[test]
fn try_call_non_array_second_arg_is_type_error() {
    // try_call(add, 5): the args parameter must be an Array. This is try_call's
    // own type check, so nothing is invoked and the error propagates uncaught.
    let add = named("add", vec![Bytecode::Pop], Arity::Exact(0), vec![], vec![]);
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::PushInt(5),
            Bytecode::Call(2),
        ],
        Arity::Exact(0),
        vec![],
        vec![add],
    );
    let err = run(program).unwrap_err();
    assert!(err.message().contains("Array"), "got: {}", err.message());
}

// ============================================================
// try_call: catching each error source
// ============================================================

#[test]
fn try_call_catches_division_by_zero() {
    let fail = named("fail", div_zero_body(), Arity::Exact(0), vec![], vec![]);
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1),
        ],
        Arity::Exact(0),
        vec![],
        vec![fail],
    );
    let result = run(program).unwrap();
    let map = expect_map(result.tail());
    assert_eq!(map.get_str("ok"), Some(&Value::Bool(false)));
    assert_eq!(map.get_str("error"), Some(&Value::from("Division by zero")));
    assert_eq!(trace_of(map), vec!["fail"]);
    // Exactly { ok, error, trace } on failure.
    assert_eq!(map.len(), 3);
    assert!(map.get_str("value").is_none());
}

#[test]
fn try_call_non_function_first_arg_is_type_error() {
    // try_call(42): the function parameter is type-checked up front (checked_native),
    // so a non-function is try_call's own error and propagates uncaught -- it never
    // reaches the catch, so there is no failure map.
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            Bytecode::PushInt(42),
            Bytecode::Call(1),
        ],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let err = run(program).unwrap_err();
    assert!(err.message().contains("Function"), "got: {}", err.message());
    assert!(err.message().contains("Int"), "got: {}", err.message());
}

#[test]
fn try_call_surfaces_a_non_string_thrown_value() {
    // A function raises a non-string value (an Int) via `ProduceError`; `try_call` must
    // surface it as that exact Value, proving `FrostError` carried the arbitrary payload
    // through rather than stringifying it.
    let raiser = named(
        "raiser",
        vec![
            Bytecode::Pop, // drop the closure's own function value
            Bytecode::PushInt(42),
            Bytecode::ProduceError,
        ],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1),
        ],
        Arity::Exact(0),
        vec![],
        vec![raiser],
    );
    let result = run(program).unwrap();
    let map = expect_map(result.tail());
    assert_eq!(map.get_str("ok"), Some(&Value::Bool(false)));
    // The caught error is the Int itself, not a rendered string.
    assert_eq!(map.get_str("error"), Some(&Value::Int(42)));
    assert_eq!(trace_of(map), vec!["raiser"]);
}

#[test]
fn try_call_catches_arity_mismatch() {
    // try_call(identity) with no further args; identity wants 1.
    let identity = named(
        "identity",
        vec![Bytecode::DefLocal(0), Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::Exact(1),
        vec![entry("x", false)],
        vec![],
    );
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1),
        ],
        Arity::Exact(0),
        vec![],
        vec![identity],
    );
    let result = run(program).unwrap();
    let map = expect_map(result.tail());
    assert_eq!(map.get_str("ok"), Some(&Value::Bool(false)));
    assert!(
        map.get_str("error")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("expects 1"),
        "unexpected error: {:?}",
        map.get_str("error")
    );
    // The callee never ran, so no frame is recorded.
    assert!(trace_of(map).is_empty());
}

#[test]
fn try_call_catches_native_error() {
    // try_call(boom): the catchee is a native that raises; its name appears.
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            Bytecode::LoadLocal(0),
            Bytecode::Call(1),
        ],
        Arity::Exact(0),
        vec![entry("boom", false)],
        vec![],
    );
    let result = run_with(program, vec![("boom", boom_native())]).unwrap();
    let map = expect_map(result.tail());
    assert_eq!(map.get_str("ok"), Some(&Value::Bool(false)));
    assert_eq!(map.get_str("error"), Some(&Value::from("boom!")));
    assert_eq!(trace_of(map), vec!["boom"]);
}

#[test]
fn try_call_catches_native_arity_mismatch() {
    // try_call(boom, [5]): boom is Exact(0). The arity check fails inside run_native
    // before the body runs (its own early-return path), and is caught.
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            Bytecode::LoadLocal(0), // boom
            Bytecode::PushInt(5),   // one arg too many
            Bytecode::MakeArray(1),
            Bytecode::Call(2), // try_call(boom, [5])
        ],
        Arity::Exact(0),
        vec![entry("boom", false)],
        vec![],
    );
    let result = run_with(program, vec![("boom", boom_native())]).unwrap();
    let map = expect_map(result.tail());
    assert_eq!(map.get_str("ok"), Some(&Value::Bool(false)));
    assert!(
        map.get_str("error")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("expects 0"),
        "unexpected error: {:?}",
        map.get_str("error")
    );
    // The body never ran, so no frame (not even boom's) is recorded.
    assert!(trace_of(map).is_empty());
}

#[test]
fn try_call_backtrace_through_nested_frames() {
    // try_call(outer); outer -> inner; inner divides by zero.
    let inner = named("inner", div_zero_body(), Arity::Exact(0), vec![], vec![]);
    let outer = named(
        "outer",
        vec![Bytecode::Pop, closure(0), Bytecode::Call(0)],
        Arity::Exact(0),
        vec![],
        vec![inner],
    );
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1),
        ],
        Arity::Exact(0),
        vec![],
        vec![outer],
    );
    let result = run(program).unwrap();
    let map = expect_map(result.tail());
    assert_eq!(map.get_str("ok"), Some(&Value::Bool(false)));
    // try_call itself is NOT in the trace (it caught, it did not `?`).
    assert_eq!(trace_of(map), vec!["inner", "outer"]);
}

// ============================================================
// Error flowing THROUGH a non-catching native frame
// ============================================================

#[test]
fn error_propagates_through_native_to_top_level() {
    // apply(fail) at top level: apply re-enters the VM, fail raises, apply
    // propagates, the error reaches run(). The native's name is in the trace.
    let fail = named("fail", div_zero_body(), Arity::Exact(0), vec![], vec![]);
    let program = named(
        "main",
        vec![Bytecode::LoadLocal(0), closure(0), Bytecode::Call(1)],
        Arity::Exact(0),
        vec![entry("apply", false)],
        vec![fail],
    );
    let err = run_with(program, vec![("apply", apply_native())]).unwrap_err();
    assert_eq!(err.message(), "Division by zero");
    assert_eq!(backtrace(&err), vec!["fail", "apply", "main"]);
}

#[test]
fn try_call_catches_error_through_native() {
    // try_call(apply, [fail]): error crosses the apply native frame and is caught.
    let fail = named("fail", div_zero_body(), Arity::Exact(0), vec![], vec![]);
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            Bytecode::LoadLocal(0), // apply
            closure(0),             // fail
            Bytecode::MakeArray(1),
            Bytecode::Call(2), // try_call(apply, [fail])
        ],
        Arity::Exact(0),
        vec![entry("apply", false)],
        vec![fail],
    );
    let result = run_with(program, vec![("apply", apply_native())]).unwrap();
    let map = expect_map(result.tail());
    assert_eq!(map.get_str("ok"), Some(&Value::Bool(false)));
    assert_eq!(trace_of(map), vec!["fail", "apply"]);
}

#[test]
fn handler_nested_in_repropagating_native_is_innermost_catcher() {
    // Vm -> apply (re-propagate) -> Vm closure C -> try_call (handle) -> Vm fail (error).
    // The innermost native (try_call) catches, so no error escapes apply: the
    // whole program returns Ok with the failure map that C produced.
    let fail = named("fail", div_zero_body(), Arity::Exact(0), vec![], vec![]);
    let c = named(
        "c",
        vec![
            Bytecode::Pop,
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),        // fail (c's child)
            Bytecode::Call(1), // try_call(fail): caught here, returns the map
        ],
        Arity::Exact(0),
        vec![],
        vec![fail],
    );
    let program = named(
        "main",
        vec![
            Bytecode::LoadLocal(0), // apply
            closure(0),             // c (main's child)
            Bytecode::Call(1),      // apply(c)
        ],
        Arity::Exact(0),
        vec![entry("apply", false)],
        vec![c],
    );
    // `.unwrap()`: the error did NOT escape; the inner try_call caught it.
    let result = run_with(program, vec![("apply", apply_native())]).unwrap();
    let map = expect_map(result.tail());
    assert_eq!(map.get_str("ok"), Some(&Value::Bool(false)));
    assert_eq!(trace_of(map), vec!["fail"]);
}

#[test]
fn error_repropagates_through_two_native_frames() {
    // Vm -> apply (re-propagate) -> Vm -> apply (re-propagate) -> Vm fail (error).
    // No handler anywhere: the error crosses BOTH apply frames (re-propagating
    // twice) and reaches run(). Both native names appear in the trace.
    let fail = named("fail", div_zero_body(), Arity::Exact(0), vec![], vec![]);
    let program = named(
        "main",
        vec![
            Bytecode::LoadLocal(0), // apply (outer)
            Bytecode::LoadLocal(0), // apply (inner: outer's first arg)
            closure(0),             // fail
            Bytecode::Call(2),      // apply(apply, fail)
        ],
        Arity::Exact(0),
        vec![entry("apply", false)],
        vec![fail],
    );
    let err = run_with(program, vec![("apply", apply_native())]).unwrap_err();
    assert_eq!(err.message(), "Division by zero");
    assert_eq!(backtrace(&err), vec!["fail", "apply", "apply", "main"]);
}

// ============================================================
// Stack hygiene: the boundary truncates to its floor on unwind
// ============================================================

#[test]
fn catch_truncates_abandoned_operands() {
    // A sentinel sits below the try_call. The failing closure pushes operand
    // garbage (1, 2, 3) before raising; if the boundary truncates correctly, the
    // garbage is gone and the sentinel survives the catch.
    let messy = named(
        "messy",
        vec![
            Bytecode::Pop,
            Bytecode::PushInt(1),
            Bytecode::PushInt(2),
            Bytecode::PushInt(3), // operand garbage
            Bytecode::PushInt(5),
            Bytecode::PushInt(0),
            Bytecode::Divide, // raise, leaving the garbage behind
        ],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let program = named(
        "main",
        vec![
            Bytecode::PushInt(777), // sentinel
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1), // catch
            Bytecode::Pop,     // discard the result map
        ],
        Arity::Exact(0),
        vec![],
        vec![messy],
    );
    let result = run(program).unwrap();
    assert_eq!(result.tail(), &Value::Int(777));
}

#[test]
fn vm_continues_after_catch() {
    // After a caught error, ordinary work proceeds on the same Vm.
    let fail = named("fail", div_zero_body(), Arity::Exact(0), vec![], vec![]);
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1), // catch
            Bytecode::Pop,     // discard result
            Bytecode::PushInt(10),
            Bytecode::PushInt(20),
            Bytecode::Add, // 30
        ],
        Arity::Exact(0),
        vec![],
        vec![fail],
    );
    let result = run(program).unwrap();
    assert_eq!(result.tail(), &Value::Int(30));
}

#[test]
fn frame_stack_restored_after_catch() {
    // After a catch, a fresh closure call must work normally. A frame leaked by
    // the unwind would corrupt `this_frame()`/base math, so `identity(7)` would
    // not yield 7 (it would run the wrong code or mis-seat the arg).
    let fail = named("fail", div_zero_body(), Arity::Exact(0), vec![], vec![]);
    let identity = named(
        "identity",
        vec![Bytecode::DefLocal(0), Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::Exact(1),
        vec![entry("x", false)],
        vec![],
    );
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),        // fail
            Bytecode::Call(1), // try_call(fail): catches
            Bytecode::Pop,     // discard result map
            closure(1),        // identity
            Bytecode::PushInt(7),
            Bytecode::Call(1), // identity(7) -> 7, only correct if frames were restored
        ],
        Arity::Exact(0),
        vec![],
        vec![fail, identity],
    );
    let result = run(program).unwrap();
    assert_eq!(result.tail(), &Value::Int(7));
}

#[test]
fn native_arg_pool_reused_after_catch() {
    // The error path recycles its native arg buffer; a subsequent native call
    // must still receive its args correctly.
    let fail = named("fail", div_zero_body(), Arity::Exact(0), vec![], vec![]);
    let add = native("add", Arity::Exact(2), |_ctx, args| {
        Ok(Value::from(
            args[0].as_int().unwrap() + args[1].as_int().unwrap(),
        ))
    });
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1), // try_call(fail): catches, recycles a buffer
            Bytecode::Pop,
            Bytecode::LoadLocal(0), // add
            Bytecode::PushInt(2),
            Bytecode::PushInt(3),
            Bytecode::Call(2), // add(2, 3): reuses a pooled buffer
        ],
        Arity::Exact(0),
        vec![entry("add", false)],
        vec![fail],
    );
    let result = run_with(program, vec![("add", add)]).unwrap();
    assert_eq!(result.tail(), &Value::Int(5));
}

// ============================================================
// Nested and adjacent try_call
// ============================================================

#[test]
fn nested_try_call_inner_catch_is_independent() {
    // try_call(mid); mid catches its own try_call(inner_fail), then raises its
    // own error, which the outer try_call catches. The inner catch must not
    // disturb the outer unwind.
    let inner_fail = named(
        "inner_fail",
        div_zero_body(),
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let mid = named(
        "mid",
        vec![
            Bytecode::Pop,
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),        // inner_fail
            Bytecode::Call(1), // try_call(inner_fail) -> caught
            Bytecode::Pop,     // discard inner result
            Bytecode::PushInt(1),
            Bytecode::PushInt(0),
            Bytecode::Divide, // mid's own error
        ],
        Arity::Exact(0),
        vec![],
        vec![inner_fail],
    );
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1),
        ],
        Arity::Exact(0),
        vec![],
        vec![mid],
    );
    let result = run(program).unwrap();
    let map = expect_map(result.tail());
    assert_eq!(map.get_str("ok"), Some(&Value::Bool(false)));
    assert_eq!(map.get_str("error"), Some(&Value::from("Division by zero")));
    // Only mid's frame; inner_fail's error was caught and never propagated here.
    assert_eq!(trace_of(map), vec!["mid"]);
}

#[test]
fn error_after_try_call_still_propagates() {
    // try_call only catches its own invocation; a later error is uncaught.
    let safe = named(
        "safe",
        vec![Bytecode::Pop, Bytecode::PushInt(1)],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1), // try_call(safe) -> ok
            Bytecode::Pop,
            Bytecode::PushInt(1),
            Bytecode::PushInt(0),
            Bytecode::Divide, // uncaught
        ],
        Arity::Exact(0),
        vec![],
        vec![safe],
    );
    let err = run(program).unwrap_err();
    assert_eq!(err.message(), "Division by zero");
    assert_eq!(backtrace(&err), vec!["main"]);
}

// ============================================================
// TCO + error interaction
// ============================================================

#[test]
fn tail_call_chain_error_is_caught() {
    // A ->tail B ->tail C, C raises. Caught by try_call; a sentinel below proves
    // the operand stack was truncated to the boundary floor despite frame reuse.
    let c = named("C", div_zero_body(), Arity::Exact(0), vec![], vec![]);
    let b = named(
        "B",
        vec![Bytecode::Pop, closure(0), Bytecode::TailCall(0)],
        Arity::Exact(0),
        vec![],
        vec![c],
    );
    let a = named(
        "A",
        vec![Bytecode::Pop, closure(0), Bytecode::TailCall(0)],
        Arity::Exact(0),
        vec![],
        vec![b],
    );
    let program = named(
        "main",
        vec![
            Bytecode::PushInt(888), // sentinel
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1), // try_call(A)
            Bytecode::Pop,     // discard result map
        ],
        Arity::Exact(0),
        vec![],
        vec![a],
    );
    let result = run(program).unwrap();
    assert_eq!(result.tail(), &Value::Int(888));
}

#[test]
fn tail_call_chain_error_trace_is_lossy_under_tco() {
    // The same A ->tail B ->tail C chain: because tail calls reuse the frame in
    // place, A and B are physically gone by the time C raises, so the trace
    // records only C. This is inherent to TCO (documented, accepted behavior).
    let c = named("C", div_zero_body(), Arity::Exact(0), vec![], vec![]);
    let b = named(
        "B",
        vec![Bytecode::Pop, closure(0), Bytecode::TailCall(0)],
        Arity::Exact(0),
        vec![],
        vec![c],
    );
    let a = named(
        "A",
        vec![Bytecode::Pop, closure(0), Bytecode::TailCall(0)],
        Arity::Exact(0),
        vec![],
        vec![b],
    );
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1),
        ],
        Arity::Exact(0),
        vec![],
        vec![a],
    );
    let result = run(program).unwrap();
    let map = expect_map(result.tail());
    assert_eq!(map.get_str("ok"), Some(&Value::Bool(false)));
    assert_eq!(trace_of(map), vec!["C"]);
}

// ============================================================
// ProduceError: ( e -- ! ); an unconditional raise, same flow as other errors
// ============================================================

/// A capture-less child closure whose body is `error(message)`:
/// `Pop` (own fn value); `LoadConst(0)`; `ProduceError`, with `message` its sole constant.
fn raiser_fn(message: &str) -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "raiser".to_string(),
        code: vec![
            Bytecode::Pop,
            Bytecode::LoadConst(0),
            Bytecode::ProduceError,
        ],
        child_fns: Vec::new(),
        constants: vec![Value::from(message)],
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    })
}

#[test]
fn produce_error_with_string_raises_that_message() {
    // error("boom") at the top level reaches the host via run(), message intact.
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".to_string(),
        code: vec![Bytecode::LoadConst(0), Bytecode::ProduceError],
        child_fns: Vec::new(),
        constants: vec![Value::from("boom")],
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    let err = run(program).unwrap_err();
    assert_eq!(err.message(), "boom");
    assert_eq!(backtrace(&err), vec!["main"]);
}

#[test]
fn produce_error_unwinds_through_calls() {
    // main -> raiser, which raises. The error unwinds frames just like div-by-zero,
    // accumulating the backtrace innermost-first.
    let program = named(
        "main",
        vec![closure(0), Bytecode::Call(0)],
        Arity::Exact(0),
        vec![],
        vec![raiser_fn("boom")],
    );
    let err = run(program).unwrap_err();
    assert_eq!(err.message(), "boom");
    assert_eq!(backtrace(&err), vec!["raiser", "main"]);
}

#[test]
fn try_call_catches_produce_error() {
    // try_call(raiser) -> { ok: false, error: "boom", trace: ["raiser"] }.
    // Proves ProduceError raises a *recoverable* error on the same path as the rest.
    let program = named(
        "main",
        vec![
            Bytecode::LoadGlobal(try_call_slot()),
            closure(0),
            Bytecode::Call(1),
        ],
        Arity::Exact(0),
        vec![],
        vec![raiser_fn("boom")],
    );
    let result = run(program).unwrap();
    let map = expect_map(result.tail());
    assert_eq!(map.get_str("ok"), Some(&Value::Bool(false)));
    assert_eq!(map.get_str("error"), Some(&Value::from("boom")));
    assert_eq!(trace_of(map), vec!["raiser"]);
}

#[test]
fn produce_error_with_non_string_value_still_raises() {
    // Any operand raises (stack effect `( e -- ! )`). This asserts only the flow
    // (it raises and reaches the host); how the payload surfaces is covered by
    // `try_call_surfaces_a_non_string_thrown_value`.
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".to_string(),
        code: vec![Bytecode::PushInt(42), Bytecode::ProduceError],
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: Vec::new(),
        num_captures: 0,
        arity: Arity::Exact(0),
    });
    let err = run(program).unwrap_err();
    assert_eq!(backtrace(&err), vec!["main"]);
}
