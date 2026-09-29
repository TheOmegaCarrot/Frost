//! White-box tests for an invariant not observable through the public API: every
//! call's local slots live in one allocation, reused call after call and run
//! after run.
//!
//! A fresh allocation per call computes the same results, so only a direct look
//! at the private slot storage can tell the difference. A native `probe` records
//! where that storage is each time it is called.

use std::collections::BTreeMap;
use std::sync::Mutex;

use super::*;

/// Where the slot storage was (its address), and its capacity, when `probe` looked.
type Sightings = Arc<Mutex<Vec<(usize, usize)>>>;

/// A `probe` native that records a sighting of the slot storage into `log`.
fn probe(log: Sightings) -> Value {
    Value::native("probe", Arity::Exact(0), move |ctx, _| {
        let slots = &ctx.vm.slots;
        log.lock()
            .unwrap()
            .push((slots.as_ptr().addr(), slots.capacity()));
        Ok(Value::Null)
    })
}

/// A function of three locals, which it defines and then returns Null.
fn three_locals() -> Arc<CompiledFunction> {
    Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "three_locals".to_string(),
        code: vec![
            Bytecode::Pop,
            Bytecode::PushInt(1),
            Bytecode::DefLocal(0),
            Bytecode::PushInt(2),
            Bytecode::DefLocal(1),
            Bytecode::PushInt(3),
            Bytecode::DefLocal(2),
            Bytecode::PushNull,
        ],
        child_fns: Vec::new(),
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: ["a", "b", "c"]
            .map(|name| NameEntry {
                name: name.to_string(),
                exported: false,
            })
            .to_vec(),
        num_captures: 0,
        arity: Arity::Exact(0),
    })
}

/// A program that calls `three_locals` and then `probe`, `calls` times over.
/// `probe` is its capture, in slot 0.
fn calls_then_probes(calls: usize, probe: Value) -> Arc<Closure> {
    let call_then_probe = [
        Bytecode::CreateClosure(0),
        Bytecode::Call(0),
        Bytecode::Pop,
        Bytecode::LoadLocal(0),
        Bytecode::Call(0),
        Bytecode::Pop,
    ];
    let code = [Bytecode::Pop]
        .into_iter()
        .chain(
            call_then_probe
                .iter()
                .copied()
                .cycle()
                .take(call_then_probe.len() * calls),
        )
        .chain([Bytecode::PushNull])
        .collect();
    Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "main".to_string(),
        code,
        child_fns: vec![three_locals()],
        constants: Vec::new(),
        key_constants: Vec::new(),
        name_table: vec![NameEntry {
            name: "probe".to_string(),
            exported: false,
        }],
        num_captures: 1,
        arity: Arity::Exact(0),
    })
    .assert_trusted()
    .close(BTreeMap::from([("probe".to_string(), probe)]))
    .expect("`probe` is supplied")
}

#[test]
fn every_call_reuses_the_same_slot_storage() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let program = calls_then_probes(10, probe(Arc::clone(&log)));
    Vm::factory().build(program).unwrap().run().unwrap();

    let sightings = log.lock().unwrap();
    assert_eq!(sightings.len(), 10);
    assert!(
        sightings.iter().all(|sighting| *sighting == sightings[0]),
        "the storage moved or grew between calls: {sightings:?}"
    );
}

#[test]
fn a_recycled_vm_reuses_its_slot_storage() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let program = calls_then_probes(3, probe(Arc::clone(&log)));
    let result = Vm::factory()
        .build(Arc::clone(&program))
        .unwrap()
        .run()
        .unwrap();
    result.reset(program).run().unwrap();

    let sightings = log.lock().unwrap();
    assert_eq!(sightings.len(), 6);
    assert!(
        sightings.iter().all(|sighting| *sighting == sightings[0]),
        "the storage moved or grew between runs: {sightings:?}"
    );
}
