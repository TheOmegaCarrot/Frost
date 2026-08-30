//! Tests for the assembly pass: label resolution, jump-offset arithmetic,
//! draining inline payloads into their pools, metadata passthrough, and the
//! compiler-bug panics (undefined label, backward/self jump).
//!
//! Assembly is purely mechanical, so these are exhaustive and paranoid: every
//! jump kind, every payload kind, their interleavings, and the boundary cases
//! (empty body, target one past the end, large offsets).

use super::super::{FunctionBuilder, Ir, JumpType};
use crate::{CompilerOptions, OptimizationOptions};

use frost_runtime::{Arity, Bytecode, CompiledFunction, MapKey, NameEntry, Value};

use std::sync::Arc;

fn options() -> CompilerOptions {
    CompilerOptions {
        optimization_options: OptimizationOptions {
            constant_fold: false,
        },
    }
}

fn builder(options: &CompilerOptions) -> FunctionBuilder<'_> {
    // Assembly never touches the filename or source; only diagnostics do.
    FunctionBuilder::new(options, "<test>".to_string(), "", "", Arity::Exact(0))
}

/// A trivial child function, for exercising closure pooling.
fn child_fn(options: &CompilerOptions) -> Arc<CompiledFunction> {
    builder(options).assemble(vec![Ir::Ready(Bytecode::PushNull)])
}

// -- Jumps --

#[test]
fn forward_jump_offset_skips_intervening_ops() {
    let options = options();
    let mut b = builder(&options);
    let end = b.next_label();

    // if-false over two pushes, landing on the trailing PushNull.
    let f = b.assemble(vec![
        Ir::Ready(Bytecode::PushTrue),
        Ir::Jump {
            kind: JumpType::IfFalse,
            label: end,
        },
        Ir::Ready(Bytecode::PushInt(1)),
        Ir::Ready(Bytecode::PushInt(2)),
        Ir::Label(end),
        Ir::Ready(Bytecode::PushNull),
    ]);

    // Label sits at index 4 (Label is zero-width); the jump sits at index 1,
    // so it must skip 4 - (1 + 1) = 2 instructions.
    assert_eq!(
        f.code,
        vec![
            Bytecode::PushTrue,
            Bytecode::JumpIfFalse(2),
            Bytecode::PushInt(1),
            Bytecode::PushInt(2),
            Bytecode::PushNull,
        ],
        "labels drop out and the jump offset lands on PushNull"
    );
}

#[test]
fn jump_to_immediately_following_label_is_zero_offset() {
    let options = options();
    let mut b = builder(&options);
    let here = b.next_label();

    let f = b.assemble(vec![
        Ir::Jump {
            kind: JumpType::Unconditional,
            label: here,
        },
        Ir::Label(here),
        Ir::Ready(Bytecode::PushNull),
    ]);

    assert_eq!(
        f.code,
        vec![Bytecode::Jump(0), Bytecode::PushNull],
        "a jump to the very next instruction is Jump(0)"
    );
}

/// Assemble a single jump-of-`kind` over one instruction to a label, returning
/// the lowered code. The offset is always 1 (skip the lone PushInt).
fn single_jump_code(kind: JumpType) -> Vec<Bytecode> {
    let options = options();
    let mut b = builder(&options);
    let target = b.next_label();
    b.assemble(vec![
        Ir::Jump {
            kind,
            label: target,
        },
        Ir::Ready(Bytecode::PushInt(1)),
        Ir::Label(target),
        Ir::Ready(Bytecode::PushNull),
    ])
    .code
    .clone()
}

#[test]
fn every_jump_kind_maps_to_its_opcode() {
    assert_eq!(
        single_jump_code(JumpType::Unconditional)[0],
        Bytecode::Jump(1)
    );
    assert_eq!(
        single_jump_code(JumpType::IfTrue)[0],
        Bytecode::JumpIfTrue(1)
    );
    assert_eq!(
        single_jump_code(JumpType::IfFalse)[0],
        Bytecode::JumpIfFalse(1)
    );
    assert_eq!(
        single_jump_code(JumpType::PeekIfTrue)[0],
        Bytecode::PeekJumpIfTrue(1)
    );
    assert_eq!(
        single_jump_code(JumpType::PeekIfFalse)[0],
        Bytecode::PeekJumpIfFalse(1)
    );
}

#[test]
fn multiple_jumps_to_one_label_get_distinct_offsets() {
    let options = options();
    let mut b = builder(&options);
    let end = b.next_label();

    let f = b.assemble(vec![
        Ir::Jump {
            kind: JumpType::IfTrue,
            label: end,
        },
        Ir::Ready(Bytecode::PushInt(1)),
        Ir::Jump {
            kind: JumpType::IfFalse,
            label: end,
        },
        Ir::Ready(Bytecode::PushInt(2)),
        Ir::Label(end),
        Ir::Ready(Bytecode::PushNull),
    ]);

    // end is at index 4. From site 0: 4 - 1 = 3. From site 2: 4 - 3 = 1.
    assert_eq!(
        f.code,
        vec![
            Bytecode::JumpIfTrue(3),
            Bytecode::PushInt(1),
            Bytecode::JumpIfFalse(1),
            Bytecode::PushInt(2),
            Bytecode::PushNull,
        ],
        "each site computes its own offset to the shared target"
    );
}

#[test]
fn target_one_past_the_end_is_permitted() {
    let options = options();
    let mut b = builder(&options);
    let end = b.next_label();

    // The label trails every instruction, so its index equals the code length:
    // a jump there falls off the end (the VM treats that as a return).
    let f = b.assemble(vec![
        Ir::Jump {
            kind: JumpType::Unconditional,
            label: end,
        },
        Ir::Ready(Bytecode::PushInt(1)),
        Ir::Ready(Bytecode::PushInt(2)),
        Ir::Label(end),
    ]);

    // end is at index 3 (== len). From site 0: 3 - 1 = 2.
    assert_eq!(
        f.code,
        vec![
            Bytecode::Jump(2),
            Bytecode::PushInt(1),
            Bytecode::PushInt(2)
        ],
        "a trailing label resolves to one past the last instruction"
    );
}

// -- Payload pools --

#[test]
fn inline_payloads_drain_into_their_pools() {
    let options = options();
    let f = builder(&options).assemble(vec![
        Ir::Const(Value::Int(42)),
        Ir::Const(Value::Bool(true)),
        Ir::KeyIndex(MapKey::from("name")),
        Ir::Ready(Bytecode::Pop),
    ]);

    assert_eq!(
        f.code,
        vec![
            Bytecode::LoadConst(0),
            Bytecode::LoadConst(1),
            Bytecode::HardIndexMap(0),
            Bytecode::Pop,
        ],
        "ops reference the slots assigned in emission order"
    );
    assert!(matches!(f.constants[0], Value::Int(42)));
    assert!(matches!(f.constants[1], Value::Bool(true)));
    assert_eq!(f.constants.len(), 2, "no dedup: one slot per inline const");
    assert_eq!(f.key_constants, vec![MapKey::from("name")]);
}

#[test]
fn closures_drain_into_child_fns() {
    let options = options();
    let child = child_fn(&options);

    let f = builder(&options).assemble(vec![Ir::Closure {
        function: child.clone(),
        num_captures: 2,
    }]);

    assert_eq!(
        f.code,
        vec![Bytecode::CreateClosure(0)],
        "the closure op references child slot 0"
    );
    assert_eq!(f.child_fns.len(), 1);
    assert!(
        Arc::ptr_eq(&f.child_fns[0], &child),
        "the exact child Arc is pooled, not a copy"
    );
}

#[test]
fn pool_counters_are_independent_under_interleaving() {
    let options = options();
    let child = child_fn(&options);

    let f = builder(&options).assemble(vec![
        Ir::Const(Value::Int(10)),
        Ir::KeyIndex(MapKey::from("a")),
        Ir::Const(Value::Bool(true)),
        Ir::Closure {
            function: child.clone(),
            num_captures: 2,
        },
        Ir::KeyIndex(MapKey::from("b")),
        Ir::Const(Value::Int(20)),
    ]);

    // Each pool advances on its own; interleaving must not cross indices.
    assert_eq!(
        f.code,
        vec![
            Bytecode::LoadConst(0),
            Bytecode::HardIndexMap(0),
            Bytecode::LoadConst(1),
            Bytecode::CreateClosure(0),
            Bytecode::HardIndexMap(1),
            Bytecode::LoadConst(2),
        ]
    );
    assert_eq!(f.constants.len(), 3);
    assert_eq!(f.key_constants, vec![MapKey::from("a"), MapKey::from("b")]);
    assert_eq!(f.child_fns.len(), 1);
}

#[test]
fn payload_ops_count_toward_jump_offsets() {
    let options = options();
    let child = child_fn(&options);
    let mut b = builder(&options);
    let end = b.next_label();

    // A jump over a Const, a KeyIndex, and a Closure must skip all three: each
    // lowers to exactly one instruction, so they count like any other op.
    let f = b.assemble(vec![
        Ir::Jump {
            kind: JumpType::Unconditional,
            label: end,
        },
        Ir::Const(Value::Int(1)),
        Ir::KeyIndex(MapKey::from("k")),
        Ir::Closure {
            function: child,
            num_captures: 0,
        },
        Ir::Label(end),
        Ir::Ready(Bytecode::PushNull),
    ]);

    // end is at index 4. From site 0: 4 - 1 = 3.
    assert_eq!(
        f.code,
        vec![
            Bytecode::Jump(3),
            Bytecode::LoadConst(0),
            Bytecode::HardIndexMap(0),
            Bytecode::CreateClosure(0),
            Bytecode::PushNull,
        ]
    );
}

// -- Metadata and boundaries --

#[test]
fn empty_body_assembles_to_an_empty_function() {
    let options = options();
    let f = builder(&options).assemble(vec![]);

    assert!(f.code.is_empty(), "no instructions");
    assert!(f.constants.is_empty());
    assert!(f.key_constants.is_empty());
    assert!(f.child_fns.is_empty());
}

#[test]
fn a_body_of_only_labels_is_empty_code() {
    let options = options();
    let mut b = builder(&options);
    let (l0, l1) = (b.next_label(), b.next_label());

    let f = b.assemble(vec![Ir::Label(l0), Ir::Label(l1)]);

    assert!(
        f.code.is_empty(),
        "labels are zero-width and emit no instructions"
    );
}

#[test]
fn function_metadata_passes_through() {
    let options = options();
    let mut b = FunctionBuilder::new(&options, "greet".to_string(), "", "", Arity::Between(1, 3));
    b.num_captures = 1;
    b.name_table.push(NameEntry {
        name: "captured".to_string(),
        exported: false,
    });
    b.name_table.push(NameEntry {
        name: "result".to_string(),
        exported: true,
    });

    let f = b.assemble(vec![Ir::Ready(Bytecode::PushNull)]);

    assert_eq!(f.name, "greet");
    assert_eq!(f.arity, Arity::Between(1, 3));
    assert_eq!(f.num_captures, 1);
    assert_eq!(f.name_table.len(), 2);
    assert_eq!(f.name_table[1].name, "result");
    assert!(f.name_table[1].exported);
}

// -- Compiler-bug panics --

#[test]
#[should_panic(expected = "never emitted")]
fn jump_to_undefined_label_panics() {
    let options = options();
    let mut b = builder(&options);
    let dangling = b.next_label(); // minted but never placed

    let _ = b.assemble(vec![
        Ir::Jump {
            kind: JumpType::Unconditional,
            label: dangling,
        },
        Ir::Ready(Bytecode::PushNull),
    ]);
}

#[test]
#[should_panic(expected = "only jumps forward")]
fn backward_jump_panics() {
    let options = options();
    let mut b = builder(&options);
    let start = b.next_label();

    let _ = b.assemble(vec![
        Ir::Label(start),
        Ir::Ready(Bytecode::PushInt(1)),
        Ir::Jump {
            kind: JumpType::Unconditional,
            label: start,
        },
    ]);
}

#[test]
#[should_panic(expected = "only jumps forward")]
fn self_jump_panics() {
    let options = options();
    let mut b = builder(&options);
    let here = b.next_label();

    // The label precedes the jump itself: the jump targets its own slot.
    let _ = b.assemble(vec![
        Ir::Label(here),
        Ir::Jump {
            kind: JumpType::Unconditional,
            label: here,
        },
    ]);
}

// -- Torture --

#[test]
fn long_forward_jump_offset() {
    const N: usize = 1000;
    let options = options();
    let mut b = builder(&options);
    let end = b.next_label();

    let mut code = vec![Ir::Jump {
        kind: JumpType::Unconditional,
        label: end,
    }];
    code.extend((0..N).map(|i| Ir::Ready(Bytecode::PushInt(i as i64))));
    code.push(Ir::Label(end));
    code.push(Ir::Ready(Bytecode::PushNull));

    let f = b.assemble(code);

    // Jump at 0; end trails the N pushes, at index 1 + N. Offset = N.
    assert_eq!(f.code[0], Bytecode::Jump(N));
    assert_eq!(f.code.len(), N + 2, "jump + N pushes + PushNull");
    assert_eq!(*f.code.last().unwrap(), Bytecode::PushNull);
}

#[test]
fn dense_mix_of_jumps_labels_and_payloads() {
    let options = options();
    let child = child_fn(&options);
    let mut b = builder(&options);
    let a = b.next_label();
    let z = b.next_label();

    let f = b.assemble(vec![
        Ir::Jump {
            kind: JumpType::IfFalse,
            label: z,
        }, // 0
        Ir::Const(Value::Int(7)), // 1
        Ir::Jump {
            kind: JumpType::Unconditional,
            label: a,
        }, // 2
        Ir::KeyIndex(MapKey::from("k")), // 3
        Ir::Label(a),             // -> index 4
        Ir::Closure {
            function: child.clone(),
            num_captures: 1,
        }, // 4
        Ir::Ready(Bytecode::Pop), // 5
        Ir::Label(z),             // -> index 6
        Ir::Ready(Bytecode::PushNull), // 6
    ]);

    // a is at index 4, z at index 6.
    // site 0 -> z: 6 - 1 = 5. site 2 -> a: 4 - 3 = 1.
    assert_eq!(
        f.code,
        vec![
            Bytecode::JumpIfFalse(5),
            Bytecode::LoadConst(0),
            Bytecode::Jump(1),
            Bytecode::HardIndexMap(0),
            Bytecode::CreateClosure(0),
            Bytecode::Pop,
            Bytecode::PushNull,
        ]
    );
    assert!(matches!(f.constants[0], Value::Int(7)));
    assert_eq!(f.key_constants, vec![MapKey::from("k")]);
    assert_eq!(f.child_fns.len(), 1);
}

#[test]
fn convoluted_program_keeps_every_pool_and_jump_straight() {
    let options = options();
    let child_a = child_fn(&options);
    let child_b = child_fn(&options);
    let mut b = builder(&options);
    let (l1, l2, l3, l4) = (
        b.next_label(),
        b.next_label(),
        b.next_label(),
        b.next_label(),
    );

    // Every jump kind, two closures, three consts, three keys, several jumps
    // that skip over intervening (zero-width) labels, and two jumps sharing one
    // target. The right column is the final instruction index of each op; a
    // `Label -> N` line binds the label to the next index without occupying one.
    let f = b.assemble(vec![
        Ir::Jump {
            kind: JumpType::IfTrue,
            label: l1,
        }, // 0  -> l1 (5)
        Ir::Jump {
            kind: JumpType::IfFalse,
            label: l2,
        }, // 1  -> l2 (7), past l1
        Ir::Const(Value::Int(10)),       // 2
        Ir::KeyIndex(MapKey::from("a")), // 3
        Ir::Closure {
            function: child_a.clone(),
            num_captures: 0,
        }, // 4
        Ir::Label(l1),                   // -> 5
        Ir::Const(Value::Int(20)),       // 5
        Ir::Jump {
            kind: JumpType::Unconditional,
            label: l3,
        }, // 6  -> l3 (12), past l2
        Ir::Label(l2),                   // -> 7
        Ir::KeyIndex(MapKey::from("b")), // 7
        Ir::Closure {
            function: child_b.clone(),
            num_captures: 2,
        }, // 8
        Ir::Jump {
            kind: JumpType::PeekIfTrue,
            label: l3,
        }, // 9  -> l3 (12), shares target with site 6
        Ir::Jump {
            kind: JumpType::PeekIfFalse,
            label: l4,
        }, // 10 -> l4 (13), past l3
        Ir::Const(Value::Bool(true)),    // 11
        Ir::Label(l3),                   // -> 12
        Ir::KeyIndex(MapKey::from("c")), // 12
        Ir::Label(l4),                   // -> 13
        Ir::Ready(Bytecode::PushNull),   // 13
    ]);

    assert_eq!(
        f.code,
        vec![
            Bytecode::JumpIfTrue(4),  // 5 - (0 + 1)
            Bytecode::JumpIfFalse(5), // 7 - (1 + 1)
            Bytecode::LoadConst(0),
            Bytecode::HardIndexMap(0),
            Bytecode::CreateClosure(0),
            Bytecode::LoadConst(1),
            Bytecode::Jump(5), // 12 - (6 + 1)
            Bytecode::HardIndexMap(1),
            Bytecode::CreateClosure(1),
            Bytecode::PeekJumpIfTrue(2),  // 12 - (9 + 1)
            Bytecode::PeekJumpIfFalse(2), // 13 - (10 + 1)
            Bytecode::LoadConst(2),
            Bytecode::HardIndexMap(2),
            Bytecode::PushNull,
        ],
        "every jump resolves independently and every pool op keeps its slot"
    );

    assert!(matches!(f.constants[0], Value::Int(10)));
    assert!(matches!(f.constants[1], Value::Int(20)));
    assert!(matches!(f.constants[2], Value::Bool(true)));
    assert_eq!(
        f.key_constants,
        vec![MapKey::from("a"), MapKey::from("b"), MapKey::from("c")]
    );
    assert_eq!(f.child_fns.len(), 2);
    assert!(Arc::ptr_eq(&f.child_fns[0], &child_a), "slot 0 is child_a");
    assert!(Arc::ptr_eq(&f.child_fns[1], &child_b), "slot 1 is child_b");
}
