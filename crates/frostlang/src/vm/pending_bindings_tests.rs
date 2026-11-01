//! White-box tests for an invariant not observable through the public API: a
//! `MatchPattern` test leaves the VM's buffer of pending bindings empty but
//! keeps its capacity, whether the test matches, mismatches, or raises, and a
//! recycled VM keeps it too.
//!
//! Reusing the buffer and allocating a fresh one give identical results, so
//! only a direct check of the private buffer can tell them apart. (That a
//! failed test leaves no reference to the tested value behind is observable,
//! and is covered black-box in `tests/vm_match_pattern.rs` and
//! `tests/compiled_patterns.rs`.)

use super::*;

/// `[a, b]`, binding slots 0 and 1.
fn pair() -> CompiledPattern {
    CompiledPattern::Array {
        elements: [binding(0), binding(1)].into(),
        length: ArrayPatternLength::Exact,
    }
}

/// `[a, 9]`, binding slot 0 before its `9` fails against `[1, 2]`.
fn pair_ending_in_nine() -> CompiledPattern {
    CompiledPattern::Array {
        elements: [binding(0), CompiledPattern::Value(PatternComparand::Int(9))].into(),
        length: ArrayPatternLength::Exact,
    }
}

/// `[a, {[k]: _}]`, with `k` read from slot 2, binding slot 0 before its
/// lookup raises for `k`, which holds `null`.
fn pair_looking_up_slot_two() -> CompiledPattern {
    CompiledPattern::Array {
        elements: [
            binding(0),
            CompiledPattern::Map {
                entries: [(
                    PatternKey::Input(PatternInput::Local(2)),
                    CompiledPattern::Binding {
                        types: None,
                        local: None,
                    },
                )]
                .into(),
                whole: None,
            },
        ]
        .into(),
        length: ArrayPatternLength::Exact,
    }
}

fn binding(slot: u32) -> CompiledPattern {
    CompiledPattern::Binding {
        types: None,
        local: Some(slot),
    }
}

/// A top-level program that pops its own fn value and sets `k` to `null`,
/// then tests `[1, 2]` against `pair`, then against `pair_ending_in_nine`,
/// then `[1, {1: 2}]` against `pair_looking_up_slot_two`, which raises.
fn program() -> Arc<Closure> {
    let pair_of_ints = [
        Bytecode::PushInt(1),
        Bytecode::PushInt(2),
        Bytecode::MakeArray(2),
    ];
    let code = [Bytecode::Pop, Bytecode::PushNull, Bytecode::DefLocal(2)]
        .into_iter()
        .chain(pair_of_ints)
        .chain([Bytecode::MatchPattern(0), Bytecode::Pop, Bytecode::Pop])
        .chain(pair_of_ints)
        .chain([Bytecode::MatchPattern(1), Bytecode::Pop, Bytecode::Pop])
        .chain([
            Bytecode::PushInt(1),
            Bytecode::PushInt(1),
            Bytecode::PushInt(2),
            Bytecode::MakeMap(1),
            Bytecode::MakeArray(2),
            Bytecode::MatchPattern(2),
        ])
        .collect();
    let slot = |name: &str| NameEntry {
        name: name.to_string(),
        exported: false,
    };
    Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".to_string(),
        origin: None,
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        patterns: vec![pair(), pair_ending_in_nine(), pair_looking_up_slot_two()],
        name_table: vec![slot("a"), slot("b"), slot("k")],
        num_captures: 0,
        arity: Arity::Exact(0),
        debug_info: None,
    })
    .assert_well_formed()
    .into_closure()
    .expect("captures nothing")
}

#[test]
fn each_pattern_test_leaves_the_buffer_empty_with_its_capacity() {
    // That the match and the mismatch left it empty, `match_pattern`'s debug
    // assertion checks as the next test starts; this checks what the raising
    // test left.
    let failed = Vm::factory().build(program()).run().unwrap_err();
    assert!(
        failed.error().message().contains("not a valid Map key"),
        "the last test raises: {}",
        failed.error().message()
    );
    let buffer = &failed.vm.pending_bindings;
    assert!(buffer.is_empty(), "bindings left over: {buffer:?}");
    assert!(
        buffer.capacity() >= 2,
        "the buffer keeps the capacity the first test grew it to: {}",
        buffer.capacity()
    );
}

#[test]
fn a_recycled_vm_keeps_the_buffers_capacity() {
    let failed = Vm::factory().build(program()).run().unwrap_err();
    let vm = failed.into_idle_vm().build(program());
    assert!(
        vm.pending_bindings.is_empty() && vm.pending_bindings.capacity() >= 2,
        "a recycled VM starts with an empty buffer, its capacity kept: {:?} of {}",
        vm.pending_bindings,
        vm.pending_bindings.capacity()
    );
}
