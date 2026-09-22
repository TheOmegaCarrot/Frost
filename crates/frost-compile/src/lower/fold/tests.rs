//! Tests for constant folding. These drive the fold machinery directly with
//! hand-built fragments, since the expression compilers that produce foldable
//! fragments are still being filled in.

use crate::lower::assemble::assemble_code;
use crate::lower::fold::{FoldVm, constant_of, value_to_ir};
use crate::lower::locals::{Locals, SlotPlan};
use crate::lower::{ExprFragment, FunctionBuilder, Ir, Label, LocalId};
use crate::{CompilerOptions, OptimizationOptions};

use frost_runtime::{Arity, Bytecode, Value};

/// A function [`Value`], for exercising the const-representability guard.
fn a_function() -> Value {
    let function = assemble_code(
        vec![Ir::Ready(Bytecode::PushNull)],
        0,
        "<f>".to_string(),
        Arity::Exact(0),
        SlotPlan::empty(),
    );
    Value::Closure(function.assert_trusted().into_closure().unwrap())
}

fn options(constant_fold: bool) -> CompilerOptions {
    CompilerOptions {
        optimization_options: OptimizationOptions {
            constant_fold,
            constant_propagate: false,
        },
        implicit_export: false,
    }
}

fn builder<'a>(options: &'a CompilerOptions, fold_vm: &'a FoldVm) -> FunctionBuilder<'a> {
    FunctionBuilder {
        locals: Locals::new(),
        next_label: Label(0),
        name: "<test>".to_string(),
        arity: Arity::Exact(0),
        source: "",
        filename: "",
        options,
        fold_vm: Some(fold_vm),
        top_level: false,
    }
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
        Some(Ir::Ready(Bytecode::PushNull))
    ));
    assert!(matches!(
        value_to_ir(Value::Bool(true)),
        Some(Ir::Ready(Bytecode::PushTrue))
    ));
    assert!(matches!(
        value_to_ir(Value::Bool(false)),
        Some(Ir::Ready(Bytecode::PushFalse))
    ));
    assert!(matches!(
        value_to_ir(Value::Int(7)),
        Some(Ir::Ready(Bytecode::PushInt(7)))
    ));
    // A string cannot be inlined, so it goes to the constant pool.
    assert!(matches!(value_to_ir(Value::from("hi")), Some(Ir::Const(_))));
}

#[test]
fn value_to_ir_rejects_functions_transitively() {
    // A bare function is not const-representable.
    assert!(value_to_ir(a_function()).is_none());
    // Nor is a structure that transitively holds one.
    let array = Value::from(vec![Value::Int(1), a_function()]);
    assert!(value_to_ir(array).is_none());
    // A function-free structure still pools.
    let clean = Value::from(vec![Value::Int(1), Value::Int(2)]);
    assert!(matches!(value_to_ir(clean), Some(Ir::Const(_))));
}

#[test]
fn constant_of_reads_a_lone_value_op() {
    // Every inline scalar push, and a pooled value.
    assert_eq!(constant_of(&[Ir::Ready(Bytecode::PushNull)]), Some(Value::Null));
    assert_eq!(
        constant_of(&[Ir::Ready(Bytecode::PushTrue)]),
        Some(Value::Bool(true))
    );
    assert_eq!(
        constant_of(&[Ir::Ready(Bytecode::PushInt(5))]),
        Some(Value::Int(5))
    );
    assert_eq!(
        constant_of(&[Ir::Const(Value::from("hi"))]),
        Some(Value::from("hi"))
    );
}

#[test]
fn constant_of_rejects_anything_but_a_lone_value_op() {
    // Empty, multi-op, and a non-value op are all not compile-time known here.
    assert_eq!(constant_of(&[]), None);
    assert_eq!(
        constant_of(&[
            Ir::Ready(Bytecode::PushInt(1)),
            Ir::Ready(Bytecode::PushInt(2)),
            Ir::Ready(Bytecode::Add),
        ]),
        None
    );
    assert_eq!(constant_of(&[Ir::LoadLocal(LocalId(0))]), None);
}

#[test]
fn folds_a_pure_arithmetic_fragment_to_an_inline_push() {
    let options = options(true);
    let fold_vm = FoldVm::new();
    let folded = builder(&options, &fold_vm).fold_if_eligible(arithmetic(Bytecode::Add));
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
            Ir::LoadLocal(LocalId(0)),
            Ir::Ready(Bytecode::PushInt(1)),
            Ir::Ready(Bytecode::Add),
        ],
        foldable: false,
    };
    let fold_vm = FoldVm::new();
    assert_eq!(builder(&options, &fold_vm).fold_if_eligible(fragment).code.len(), 3);
}

#[test]
fn folding_is_skipped_when_the_option_is_off() {
    let options = options(false);
    let fold_vm = FoldVm::new();
    assert_eq!(
        builder(&options, &fold_vm).fold_if_eligible(arithmetic(Bytecode::Add)).code.len(),
        3
    );
}

#[test]
fn a_single_op_fragment_is_not_re_folded() {
    // Already minimal: folding would only spend a VM run to reproduce it.
    let options = options(true);
    let fragment = ExprFragment {
        code: vec![Ir::Ready(Bytecode::PushInt(42))],
        foldable: true,
    };
    let fold_vm = FoldVm::new();
    let folded = builder(&options, &fold_vm).fold_if_eligible(fragment);
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
    let fold_vm = FoldVm::new();
    let folded = builder(&options, &fold_vm).fold_if_eligible(fragment);
    assert_eq!(folded.code.len(), 3, "the original bytecode is kept");
}
