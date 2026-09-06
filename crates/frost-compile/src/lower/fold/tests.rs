//! Tests for constant folding. These drive the fold machinery directly with
//! hand-built fragments, since the expression compilers that produce foldable
//! fragments are still being filled in.

use crate::lower::fold::value_to_ir;
use crate::lower::{ExprFragment, FunctionBuilder, Ir};
use crate::{CompilerOptions, OptimizationOptions};

use frost_runtime::{Arity, Bytecode, Value};

fn options(constant_fold: bool) -> CompilerOptions {
    CompilerOptions {
        optimization_options: OptimizationOptions { constant_fold },
    }
}

fn builder(options: &CompilerOptions) -> FunctionBuilder<'_> {
    FunctionBuilder::new(options, "<test>".to_string(), "", "", Arity::Exact(0))
}

/// A foldable `1 op 2` fragment.
fn arithmetic(op: Bytecode) -> ExprFragment {
    ExprFragment {
        code: vec![
            Ir::Ready(Bytecode::PushInt(1)),
            Ir::Ready(Bytecode::PushInt(2)),
            Ir::Ready(op),
        ],
        foldable: true,
    }
}

#[test]
fn value_to_ir_inlines_scalars_and_pools_structured() {
    assert!(matches!(
        value_to_ir(Value::Null),
        Ir::Ready(Bytecode::PushNull)
    ));
    assert!(matches!(
        value_to_ir(Value::Bool(true)),
        Ir::Ready(Bytecode::PushTrue)
    ));
    assert!(matches!(
        value_to_ir(Value::Bool(false)),
        Ir::Ready(Bytecode::PushFalse)
    ));
    assert!(matches!(
        value_to_ir(Value::Int(7)),
        Ir::Ready(Bytecode::PushInt(7))
    ));
    // A string cannot be inlined, so it goes to the constant pool.
    assert!(matches!(value_to_ir(Value::from("hi")), Ir::Const(_)));
}

#[test]
fn folds_a_pure_arithmetic_fragment_to_an_inline_push() {
    let options = options(true);
    let folded = builder(&options).fold(arithmetic(Bytecode::Add));
    assert_eq!(folded.code.len(), 1, "collapsed to one op");
    assert!(
        matches!(folded.code[0], Ir::Ready(Bytecode::PushInt(3))),
        "1 + 2 folded to an inline PushInt(3), not a pool load"
    );
    assert!(folded.foldable, "a folded constant is itself foldable");
}

#[test]
fn a_non_foldable_fragment_is_left_alone() {
    let options = options(true);
    let fragment = ExprFragment {
        code: vec![
            Ir::Ready(Bytecode::LoadLocal(0)),
            Ir::Ready(Bytecode::PushInt(1)),
            Ir::Ready(Bytecode::Add),
        ],
        foldable: false,
    };
    assert_eq!(builder(&options).fold(fragment).code.len(), 3);
}

#[test]
fn folding_is_skipped_when_the_option_is_off() {
    let options = options(false);
    assert_eq!(builder(&options).fold(arithmetic(Bytecode::Add)).code.len(), 3);
}

#[test]
fn a_single_op_fragment_is_not_re_folded() {
    // Already minimal: folding would only spend a VM run to reproduce it.
    let options = options(true);
    let fragment = ExprFragment {
        code: vec![Ir::Ready(Bytecode::PushInt(42))],
        foldable: true,
    };
    let folded = builder(&options).fold(fragment);
    assert!(matches!(folded.code[0], Ir::Ready(Bytecode::PushInt(42))));
    assert_eq!(folded.code.len(), 1);
}

#[test]
fn a_fragment_that_errors_when_evaluated_is_kept_as_bytecode() {
    // `1 / 0` errors; the fold is abandoned so the error surfaces at runtime.
    let options = options(true);
    let fragment = ExprFragment {
        code: vec![
            Ir::Ready(Bytecode::PushInt(1)),
            Ir::Ready(Bytecode::PushInt(0)),
            Ir::Ready(Bytecode::Divide),
        ],
        foldable: true,
    };
    let folded = builder(&options).fold(fragment);
    assert_eq!(folded.code.len(), 3, "the original bytecode is kept");
}
