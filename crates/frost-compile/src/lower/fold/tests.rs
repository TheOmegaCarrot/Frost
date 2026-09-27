//! Tests for constant folding. These drive the fold machinery directly with
//! hand-built fragments, since the expression compilers that produce foldable
//! fragments are still being filled in.

use crate::lower::assemble::assemble_code;
use crate::lower::fold::{FoldVm, constant_of, value_to_ir};
use crate::lower::locals::{Locals, SlotPlan};
use crate::lower::{ExprFragment, FunctionBuilder, Ir, Label, LocalId};
use crate::{CompilerOptions, OptimizationOptions};

use frost_runtime::{Arity, Bytecode, FrostFloat, MapKey, Value};

/// A function [`Value`], for exercising the const-representability guard.
fn a_function() -> Value {
    let function = assemble_code(
        &[Ir::Ready(Bytecode::PushNull)],
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
            branch_eliminate: false,
            capture_hoist: false,
        },
        implicit_export: false,
    }
}

#[test]
fn a_fragment_ending_in_closure_creation_is_not_evaluated() {
    // Its value is a Function, which can never be a constant. This fragment
    // reads a local it never defines, so assembling it for evaluation would
    // panic: the test passing proves no evaluation is attempted.
    let options = options(true);
    let fold_vm = FoldVm::new();
    let child = builder(&options, &fold_vm).finish(vec![Ir::Ready(Bytecode::PushNull)]);
    let fragment = ExprFragment {
        code: vec![Ir::LoadLocal(LocalId(0)), Ir::closure(child)],
        foldable: true,
    };
    let result = builder(&options, &fold_vm).fold_if_eligible(fragment);
    assert_eq!(result.code.len(), 2, "the fragment is kept as is");
    assert!(result.foldable, "it stays usable within an enclosing fold");
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
        effectful: false,
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

/// A non-foldable `x + 1` fragment, reading a local.
fn local_plus_one() -> ExprFragment {
    ExprFragment {
        code: vec![
            Ir::LoadLocal(LocalId(0)),
            Ir::Ready(Bytecode::PushInt(1)),
            Ir::Ready(Bytecode::Add),
        ],
        foldable: false,
    }
}

/// Whether `fragment` was folded to the lone `PushInt(value)`.
fn is_folded_to(fragment: &ExprFragment, value: i64) -> bool {
    matches!(fragment.code.as_slice(), [Ir::Ready(Bytecode::PushInt(v))] if *v == value)
}

/// Whether `fragment` is still the unfolded `1 op 2` or `x + 1` (three ops).
fn is_intact(fragment: &ExprFragment) -> bool {
    fragment.code.len() == 3
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
    // A float is also an inline push, like the other scalars.
    let float = FrostFloat::new(1.5).expect("1.5 is a valid Frost float");
    assert!(matches!(
        value_to_ir(Value::Float(float)),
        Some(Ir::Ready(Bytecode::PushFloat(f))) if f.get() == 1.5
    ));
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
fn value_to_ir_rejects_a_map_holding_a_function() {
    // The transitive check applies to a Map's values just as it does to an
    // Array's elements.
    let map = Value::from_iter([(MapKey::from("f"), a_function())]);
    assert!(
        value_to_ir(map).is_none(),
        "a function nested in a map is not const-representable"
    );
    let clean = Value::from_iter([(MapKey::from("n"), Value::Int(1))]);
    assert!(
        matches!(value_to_ir(clean), Some(Ir::Const(_))),
        "a function-free map still pools"
    );
}

#[test]
fn constant_of_reads_a_lone_value_op() {
    // Every inline scalar push, and a pooled value.
    assert_eq!(
        constant_of(&[Ir::Ready(Bytecode::PushNull)]),
        Some(Value::Null)
    );
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
    let float = FrostFloat::new(2.5).expect("2.5 is a valid Frost float");
    assert_eq!(
        constant_of(&[Ir::Ready(Bytecode::PushFloat(float))]),
        Some(Value::Float(float))
    );
}

#[test]
fn constant_of_reads_an_empty_structure_literal() {
    assert_eq!(
        constant_of(&[Ir::Ready(Bytecode::MakeArray(0))]),
        Some(Value::from_iter(std::iter::empty::<Value>()))
    );
    assert_eq!(
        constant_of(&[Ir::Ready(Bytecode::MakeMap(0))]),
        Some(Value::from_iter(std::iter::empty::<(MapKey, Value)>()))
    );
    // A nonempty one consumes operands, so alone it is no value.
    assert_eq!(constant_of(&[Ir::Ready(Bytecode::MakeArray(1))]), None);
    assert_eq!(constant_of(&[Ir::Ready(Bytecode::MakeMap(1))]), None);
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
    assert_eq!(
        builder(&options, &fold_vm)
            .fold_if_eligible(fragment)
            .code
            .len(),
        3
    );
}

#[test]
fn folding_is_skipped_when_the_option_is_off() {
    let options = options(false);
    let fold_vm = FoldVm::new();
    assert_eq!(
        builder(&options, &fold_vm)
            .fold_if_eligible(arithmetic(Bytecode::Add))
            .code
            .len(),
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
fn folding_without_a_fold_vm_leaves_the_fragment_unchanged() {
    // A builder with no fold VM (e.g. a test that only assembles) must skip
    // evaluation entirely, regardless of eligibility or the option.
    let options = options(true);
    let no_fold_vm = FunctionBuilder {
        locals: Locals::new(),
        next_label: Label(0),
        name: "<test>".to_string(),
        arity: Arity::Exact(0),
        source: "",
        filename: "",
        options: &options,
        fold_vm: None,
        top_level: false,
        effectful: false,
    };
    let folded = no_fold_vm.fold_if_eligible(arithmetic(Bytecode::Add));
    assert_eq!(
        folded.code.len(),
        3,
        "no fold VM means no evaluation happens"
    );
    assert!(folded.foldable, "eligibility itself is unaffected");
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

// --- fold_siblings / fold_sibling_list ---
//
// The rule under test: siblings that are all foldable are left for an ancestor
// to fold whole; otherwise each foldable sibling folds now.

#[test]
fn all_foldable_siblings_are_deferred() {
    let options = options(true);
    let fold_vm = FoldVm::new();
    let builder = builder(&options, &fold_vm);

    let ([add, mul], foldable) =
        builder.fold_siblings([arithmetic(Bytecode::Add), arithmetic(Bytecode::Multiply)]);
    assert!(foldable, "all siblings foldable, so the parent is");
    assert!(is_intact(&add), "left for the parent to fold: {add:?}");
    assert!(is_intact(&mul), "left for the parent to fold: {mul:?}");

    let (list, foldable) = builder.fold_sibling_list(vec![
        arithmetic(Bytecode::Add),
        arithmetic(Bytecode::Multiply),
    ]);
    assert!(foldable, "all siblings foldable, so the parent is");
    assert!(
        list.iter().all(is_intact),
        "left for the parent to fold: {list:?}"
    );
}

#[test]
fn a_non_foldable_sibling_makes_the_foldable_ones_fold() {
    let options = options(true);
    let fold_vm = FoldVm::new();
    let builder = builder(&options, &fold_vm);

    // The foldable sibling folds on either side of the non-foldable one.
    let ([add, local], foldable) =
        builder.fold_siblings([arithmetic(Bytecode::Add), local_plus_one()]);
    assert!(
        !foldable,
        "a non-foldable sibling makes the parent non-foldable"
    );
    assert!(is_folded_to(&add, 3), "the foldable sibling folds: {add:?}");
    assert!(
        is_intact(&local),
        "the non-foldable sibling is untouched: {local:?}"
    );

    let ([local, add], foldable) =
        builder.fold_siblings([local_plus_one(), arithmetic(Bytecode::Add)]);
    assert!(!foldable);
    assert!(
        is_intact(&local),
        "the non-foldable sibling is untouched: {local:?}"
    );
    assert!(is_folded_to(&add, 3), "the foldable sibling folds: {add:?}");
}

#[test]
fn a_sibling_list_folds_each_foldable_member_in_place() {
    let options = options(true);
    let fold_vm = FoldVm::new();
    let builder = builder(&options, &fold_vm);

    let (list, foldable) = builder.fold_sibling_list(vec![
        arithmetic(Bytecode::Add),
        local_plus_one(),
        arithmetic(Bytecode::Multiply),
        local_plus_one(),
    ]);
    assert!(!foldable);
    assert_eq!(list.len(), 4, "no sibling is added or dropped");
    assert!(
        is_folded_to(&list[0], 3),
        "1 + 2 folds in place: {:?}",
        list[0]
    );
    assert!(is_intact(&list[1]), "untouched: {:?}", list[1]);
    assert!(
        is_folded_to(&list[2], 2),
        "1 * 2 folds in place: {:?}",
        list[2]
    );
    assert!(is_intact(&list[3]), "untouched: {:?}", list[3]);
}

#[test]
fn no_foldable_siblings_are_left_alone() {
    let options = options(true);
    let fold_vm = FoldVm::new();
    let builder = builder(&options, &fold_vm);

    let ([a, b], foldable) = builder.fold_siblings([local_plus_one(), local_plus_one()]);
    assert!(!foldable);
    assert!(is_intact(&a) && is_intact(&b));

    let (list, foldable) = builder.fold_sibling_list(vec![local_plus_one(), local_plus_one()]);
    assert!(!foldable);
    assert!(list.iter().all(is_intact));
}

#[test]
fn no_siblings_is_foldable() {
    // A node with no children (e.g. an empty array literal) has nothing
    // stopping it from folding.
    let options = options(true);
    let fold_vm = FoldVm::new();
    let builder = builder(&options, &fold_vm);

    let ([], foldable) = builder.fold_siblings([]);
    assert!(foldable);

    let (list, foldable) = builder.fold_sibling_list(Vec::new());
    assert!(foldable);
    assert!(list.is_empty());
}

#[test]
fn a_failing_sibling_fold_keeps_its_bytecode() {
    // `1 / 0` is due to fold, but errors, so it stays for runtime.
    let options = options(true);
    let fold_vm = FoldVm::new();
    let zero_division = ExprFragment {
        code: vec![
            Ir::Ready(Bytecode::PushInt(1)),
            Ir::Ready(Bytecode::PushInt(0)),
            Ir::Ready(Bytecode::Divide),
        ],
        foldable: true,
    };
    let ([divide, _], _) =
        builder(&options, &fold_vm).fold_siblings([zero_division, local_plus_one()]);
    assert!(
        is_intact(&divide),
        "the erroring fold is abandoned: {divide:?}"
    );
}

#[test]
fn sibling_folding_respects_the_option() {
    // With folding off, the rule still reports foldability but folds nothing.
    let options = options(false);
    let fold_vm = FoldVm::new();
    let builder = builder(&options, &fold_vm);

    let ([add, _], foldable) = builder.fold_siblings([arithmetic(Bytecode::Add), local_plus_one()]);
    assert!(!foldable);
    assert!(is_intact(&add), "folding is off: {add:?}");

    let (list, foldable) =
        builder.fold_sibling_list(vec![arithmetic(Bytecode::Add), local_plus_one()]);
    assert!(!foldable);
    assert!(is_intact(&list[0]), "folding is off: {:?}", list[0]);
}
