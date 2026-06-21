//! Correctness tests for `TailCall` into a VM closure.
//!
//! These verify the frame-reuse *mechanics* (arg slide, capture/slot setup,
//! variadic collapse, and -- crucially -- that the reused frame inherits the
//! caller's return address so the callee resumes the *original* caller). They
//! are NOT a TCO proof: at finite depth a correct reuse-the-frame TailCall and a
//! broken push-a-frame one are observably identical (same result, same resume).
//! The bound itself can only be asserted by a deep "don't go boom" test, which
//! is deferred until conditionals + arithmetic exist (a terminating recursion
//! needs a base case).
//!
//! Finite tail-call chains terminate because the last closure simply returns, so
//! no conditional is required -- everything here uses implemented opcodes only.

mod common;

use std::sync::Arc;

use common::{entry, func, run_fn};
use frost_runtime::{Arity, Bytecode, CompiledFunction, FrostArray, Value};

#[test]
fn single_tail_call() {
    // A tail-calls B; B returns 7. B inherits A's return address, so it returns
    // straight to the top level (A is elided).
    let b = func(
        vec![Bytecode::Pop, Bytecode::PushInt(7)],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let a = func(
        vec![
            Bytecode::Pop, // pop A's own function value
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::TailCall(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![b],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![a],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(7));
}

#[test]
fn chain_of_tail_calls() {
    // A ->tail B ->tail C -> 7. Three consecutive tail calls; the frame count
    // never exceeds top + one callee.
    let c = func(
        vec![Bytecode::Pop, Bytecode::PushInt(7)],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let b = func(
        vec![
            Bytecode::Pop,
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::TailCall(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![c],
    );
    let a = func(
        vec![
            Bytecode::Pop,
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::TailCall(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![b],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![a],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(7));
}

#[test]
fn tail_call_carries_args() {
    // A tail-calls identity B with 42; the arg must survive the frame reuse and
    // land in B's slot 0.
    let b = func(
        vec![Bytecode::DefLocal(0), Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::Exact(1),
        vec![entry("x", false)],
        vec![],
    );
    let a = func(
        vec![
            Bytecode::Pop,
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::PushInt(42),
            Bytecode::TailCall(1),
        ],
        Arity::Exact(0),
        vec![],
        vec![b],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![a],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(42));
}

#[test]
fn call_then_tail_call_returns_to_original_caller() {
    // f CALLS g (not a tail call); g TAIL-calls h. h must return to f -- the
    // frame g elided -- so f resumes its post-call instructions and yields 99.
    let h = func(
        vec![Bytecode::Pop, Bytecode::PushInt(7)],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let g = func(
        vec![
            Bytecode::Pop,
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::TailCall(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![h],
    );
    let f = func(
        vec![
            Bytecode::Pop, // pop f's own function value
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(0), // g() -- g tail-calls h, whose result returns HERE
            Bytecode::Pop,     // discard h's result (7)
            Bytecode::PushInt(99),
        ],
        Arity::Exact(0),
        vec![],
        vec![g],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![f],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(99));
}

#[test]
fn tail_call_leaves_exactly_one_value() {
    // A sentinel sits below the top-level call. A tail-calls B (-> 7); after the
    // call leaves exactly one result, Pop reveals the sentinel -- so the
    // tail-calling callee still honored `( f -- r )` and the top level resumed.
    let b = func(
        vec![Bytecode::Pop, Bytecode::PushInt(7)],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let a = func(
        vec![
            Bytecode::Pop,
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::TailCall(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![b],
    );
    let program = func(
        vec![
            Bytecode::PushInt(11), // sentinel
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(0),
            Bytecode::Pop, // discard the single result
        ],
        Arity::Exact(0),
        vec![],
        vec![a],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(11));
}

#[test]
fn variadic_tail_call() {
    // A tail-calls a variadic B with (1, 2, 3); the rest-collapse must run
    // through the reuse path.
    let b = func(
        vec![Bytecode::DefLocal(0), Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::AtLeast(0),
        vec![entry("rest", false)],
        vec![],
    );
    let a = func(
        vec![
            Bytecode::Pop,
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::PushInt(1),
            Bytecode::PushInt(2),
            Bytecode::PushInt(3),
            Bytecode::TailCall(3),
        ],
        Arity::Exact(0),
        vec![],
        vec![b],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![a],
    );
    let result = run_fn(program);
    let expected = Value::Array(FrostArray::from(vec![
        Value::Int(1),
        Value::Int(2),
        Value::Int(3),
    ]));
    assert_eq!(result.tail(), &expected);
}

#[test]
fn tail_call_to_closure_with_capture() {
    // A creates B with a capture, then tail-calls it. The capture must seat into
    // the reused frame's slot 0.
    let b = func(
        vec![Bytecode::Pop, Bytecode::LoadLocal(0)], // pop f, return the capture
        Arity::Exact(0),
        vec![entry("c", false)],
        vec![],
    );
    let a = func(
        vec![
            Bytecode::Pop,         // pop A's own function value
            Bytecode::PushInt(55), // value to capture
            Bytecode::CreateClosure {
                num_captures: 1,
                function: 0,
            },
            Bytecode::TailCall(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![b],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![a],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(55));
}

#[test]
fn variadic_tail_call_splits_fixed_and_rest() {
    // A tail-calls (fn a, ...rest -> rest) with (10, 20, 30). Exercises the
    // collapse index gone_frame.base_idx + 1 + fixed through the reuse path.
    let b = func(
        vec![
            Bytecode::DefLocal(1), // rest -> slot 1
            Bytecode::DefLocal(0), // a -> slot 0
            Bytecode::Pop,
            Bytecode::LoadLocal(1),
        ],
        Arity::AtLeast(1),
        vec![entry("a", false), entry("rest", false)],
        vec![],
    );
    let a = func(
        vec![
            Bytecode::Pop,
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::PushInt(10), // a
            Bytecode::PushInt(20), // rest[0]
            Bytecode::PushInt(30), // rest[1]
            Bytecode::TailCall(3),
        ],
        Arity::Exact(0),
        vec![],
        vec![b],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![a],
    );
    let result = run_fn(program);
    let expected = Value::Array(FrostArray::from(vec![Value::Int(20), Value::Int(30)]));
    assert_eq!(result.tail(), &expected);
}

#[test]
fn tail_called_frame_makes_normal_call() {
    // A tail-calls B; B then makes a *regular* Call to C and returns C's result.
    // The reused frame must be fully functional for subsequent pushes/returns,
    // and B's inherited return address still carries C's result to the top.
    let c = func(
        vec![Bytecode::Pop, Bytecode::PushInt(7)],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let b = func(
        vec![
            Bytecode::Pop,
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(0), // normal call, in tail position of B's body
        ],
        Arity::Exact(0),
        vec![],
        vec![c],
    );
    let a = func(
        vec![
            Bytecode::Pop,
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::TailCall(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![b],
    );
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![a],
    );
    let result = run_fn(program);
    assert_eq!(result.tail(), &Value::Int(7));
}

// ============================================================
// Call-in-tail-position equivalence
// ============================================================
//
// A Call in tail position and a TailCall must produce the same value -- the only
// difference is the frame-reuse optimization. Each test runs the same closure
// shape both ways and asserts they agree (and match the expected result).

/// Run a program where closure A's body is `setup` followed by `final_op` (a
/// `Call` or `TailCall` in tail position), with `children` as A's `child_fns`.
fn tail_position_result(
    setup: Vec<Bytecode>,
    final_op: Bytecode,
    children: Vec<Arc<CompiledFunction>>,
) -> Value {
    let mut body = setup;
    body.push(final_op);
    let a = func(body, Arity::Exact(0), vec![], children);
    let program = func(
        vec![
            Bytecode::CreateClosure {
                num_captures: 0,
                function: 0,
            },
            Bytecode::Call(0),
        ],
        Arity::Exact(0),
        vec![],
        vec![a],
    );
    run_fn(program).tail().clone()
}

#[test]
fn call_eq_tail_call_no_args() {
    // fn -> 7
    let callee = func(
        vec![Bytecode::Pop, Bytecode::PushInt(7)],
        Arity::Exact(0),
        vec![],
        vec![],
    );
    let setup = vec![
        Bytecode::Pop,
        Bytecode::CreateClosure {
            num_captures: 0,
            function: 0,
        },
    ];
    let via_call = tail_position_result(setup.clone(), Bytecode::Call(0), vec![callee.clone()]);
    let via_tail = tail_position_result(setup, Bytecode::TailCall(0), vec![callee]);
    assert_eq!(via_call, via_tail);
    assert_eq!(via_call, Value::Int(7));
}

#[test]
fn call_eq_tail_call_multiple_params() {
    // fn a, b -> a  (returns the first param, so ordering matters)
    let callee = func(
        vec![
            Bytecode::DefLocal(1),
            Bytecode::DefLocal(0),
            Bytecode::Pop,
            Bytecode::LoadLocal(0),
        ],
        Arity::Exact(2),
        vec![entry("a", false), entry("b", false)],
        vec![],
    );
    let setup = vec![
        Bytecode::Pop,
        Bytecode::CreateClosure {
            num_captures: 0,
            function: 0,
        },
        Bytecode::PushInt(10),
        Bytecode::PushInt(20),
    ];
    let via_call = tail_position_result(setup.clone(), Bytecode::Call(2), vec![callee.clone()]);
    let via_tail = tail_position_result(setup, Bytecode::TailCall(2), vec![callee]);
    assert_eq!(via_call, via_tail);
    assert_eq!(via_call, Value::Int(10));
}

#[test]
fn call_eq_tail_call_capture_and_param() {
    // fn x -> capture : capture in slot 0, param in slot 1; returns the capture.
    let callee = func(
        vec![Bytecode::DefLocal(1), Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::Exact(1),
        vec![entry("c", false), entry("x", false)],
        vec![],
    );
    let setup = vec![
        Bytecode::Pop,
        Bytecode::PushInt(100), // capture value
        Bytecode::CreateClosure {
            num_captures: 1,
            function: 0,
        },
        Bytecode::PushInt(5), // arg
    ];
    let via_call = tail_position_result(setup.clone(), Bytecode::Call(1), vec![callee.clone()]);
    let via_tail = tail_position_result(setup, Bytecode::TailCall(1), vec![callee]);
    assert_eq!(via_call, via_tail);
    assert_eq!(via_call, Value::Int(100));
}

#[test]
fn call_eq_tail_call_variadic_rest() {
    let callee = func(
        vec![Bytecode::DefLocal(0), Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::AtLeast(0),
        vec![entry("rest", false)],
        vec![],
    );
    let setup = vec![
        Bytecode::Pop,
        Bytecode::CreateClosure {
            num_captures: 0,
            function: 0,
        },
        Bytecode::PushInt(1),
        Bytecode::PushInt(2),
        Bytecode::PushInt(3),
    ];
    let via_call = tail_position_result(setup.clone(), Bytecode::Call(3), vec![callee.clone()]);
    let via_tail = tail_position_result(setup, Bytecode::TailCall(3), vec![callee]);
    assert_eq!(via_call, via_tail);
    let expected = Value::Array(FrostArray::from(vec![
        Value::Int(1),
        Value::Int(2),
        Value::Int(3),
    ]));
    assert_eq!(via_call, expected);
}

#[test]
fn call_eq_tail_call_variadic_empty() {
    let callee = func(
        vec![Bytecode::DefLocal(0), Bytecode::Pop, Bytecode::LoadLocal(0)],
        Arity::AtLeast(0),
        vec![entry("rest", false)],
        vec![],
    );
    let setup = vec![
        Bytecode::Pop,
        Bytecode::CreateClosure {
            num_captures: 0,
            function: 0,
        },
    ];
    let via_call = tail_position_result(setup.clone(), Bytecode::Call(0), vec![callee.clone()]);
    let via_tail = tail_position_result(setup, Bytecode::TailCall(0), vec![callee]);
    assert_eq!(via_call, via_tail);
    assert_eq!(via_call, Value::Array(FrostArray::empty()));
}
