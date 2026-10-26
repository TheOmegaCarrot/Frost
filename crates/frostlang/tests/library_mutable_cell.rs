//! The `mutable_cell` global, from Frost source: Frost's only built-in mutable
//! state.
//!
//! `mutable_cell(initial?)` returns a cell holding `initial`, or Null without
//! one: a Map of Functions over the same state. `get()` returns the value held,
//! `exchange(value)` holds `value` and returns the value it replaces, and
//! `take()` returns the value held and leaves Null in its place, so the value is
//! no longer shared with the cell.
//! A cell never holds a Function, nor an Opaque whose type has drop glue, even
//! nested in an Array or Map.
//!
//! The harness runs every case under every optimization permutation, so each is
//! checked with and without folding the calls around a cell.

use crate::script;
use crate::unshared::unshared;

use frostlang::{FrostOpaque, SpecialFloat, Value};
use script::Script;
use script::assertions::{Library, library_assertions};

library_assertions!(Library::GLOBALS);

#[test]
fn a_cell_holds_its_initial_value() {
    assert_values(&[
        ("mutable_cell().get()", "null"),
        ("mutable_cell(null).get()", "null"),
        ("mutable_cell(5).get()", "5"),
        ("mutable_cell([1, {a: 'b'}]).get()", "[1, {a: 'b'}]"),
    ]);
}

#[test]
fn a_cell_is_a_map_of_get_exchange_and_take() {
    assert_values(&[
        (
            "sorted(keys(mutable_cell()))",
            "['exchange', 'get', 'take']",
        ),
        ("is_function(mutable_cell().get)", "true"),
        ("is_function(mutable_cell().exchange)", "true"),
        ("is_function(mutable_cell().take)", "true"),
    ]);
}

#[test]
fn exchange_returns_the_value_it_replaces() {
    let source = r"
        def cell = mutable_cell('first')
        def replaced = cell.exchange('second')
        [replaced, cell.get()]
    ";
    assert_values(&[(source, "['first', 'second']")]);
}

#[test]
fn successive_exchanges_each_see_the_last() {
    let source = r"
        def cell = mutable_cell()
        [cell.exchange(1), cell.exchange([2]), cell.exchange(null), cell.get()]
    ";
    assert_values(&[(source, "[null, 1, [2], null]")]);
}

#[test]
fn get_does_not_change_the_cell() {
    let source = r"
        def cell = mutable_cell(3)
        [cell.get(), cell.get(), cell.get()]
    ";
    assert_values(&[(source, "[3, 3, 3]")]);
}

#[test]
fn take_returns_the_value_and_leaves_null() {
    let source = r"
        def cell = mutable_cell('held')
        def taken = cell.take()
        [taken, cell.get()]
    ";
    assert_values(&[
        (source, "['held', null]"),
        ("mutable_cell().take()", "null"),
    ]);
}

#[test]
fn a_cell_holds_values_again_after_a_take() {
    let source = r"
        def cell = mutable_cell(1)
        [cell.take(), cell.take(), cell.exchange(2), cell.take(), cell.get()]
    ";
    assert_values(&[(source, "[1, null, null, 2, null]")]);
}

/// A value the cell alone holds is uniquely owned once taken, so it can be updated
/// in place, where `get` leaves it shared with the cell.
/// The Array holds the host's `n`, so it is built at runtime rather than shared
/// with the constant pool.
#[test]
fn a_taken_value_is_no_longer_shared_with_the_cell() {
    let source = r"
        def cell = mutable_cell([n])
        [unshared(cell.get()), unshared(cell.take())]
    ";
    let tail = Script::new(source)
        .captures(&[("n", Value::from(1)), ("unshared", unshared())])
        .run();
    assert_eq!(
        tail,
        Value::array([false, true]),
        "`get` should leave the Array shared with the cell, and `take` should leave it held by nothing else"
    );
}

#[test]
fn a_taken_value_can_be_updated_and_exchanged_back() {
    let source = r"
        def cell = mutable_cell([1])
        cell.exchange(cell.take() + [2])
        cell.get()
    ";
    assert_values(&[(source, "[1, 2]")]);
}

#[test]
fn a_cell_holds_any_value_but_a_function() {
    for value in [
        "true",
        "1.5",
        "'text'",
        "x'00ff'",
        "[]",
        "{}",
        "[[1], {k: [null]}]",
        "{[1]: {[x'00']: 'deep'}}",
    ] {
        let source = format!(
            r"
            def cell = mutable_cell()
            cell.exchange({value})
            cell.get()
            "
        );
        assert_values(&[(&source, value)]);
    }
}

#[test]
fn every_copy_of_a_cell_shares_its_value() {
    let source = r"
        def cell = mutable_cell(0)
        def alias = cell
        def holder = {cell: cell}
        alias.exchange(1)
        def after_exchange = [cell.get(), holder.cell.get()]
        holder.cell.take()
        [after_exchange, [cell.get(), alias.get()]]
    ";
    assert_values(&[(source, "[[1, 1], [null, null]]")]);
}

#[test]
fn separate_cells_are_independent() {
    let source = r"
        def a = mutable_cell(0)
        def b = mutable_cell(0)
        a.exchange(1)
        [a.get(), b.get()]
    ";
    assert_values(&[(source, "[1, 0]")]);
}

#[test]
fn a_function_over_a_cell_sees_each_change() {
    let source = r"
        def count = mutable_cell(0)
        defn bump() -> count.exchange(count.get() + 1)
        bump()
        bump()
        each(range(3), fn _ -> bump())
        count.get()
    ";
    assert_values(&[(source, "5")]);
}

#[test]
fn a_value_read_from_a_cell_is_unchanged_by_a_later_exchange() {
    let source = r"
        def cell = mutable_cell({k: [1]})
        def read = cell.get()
        cell.exchange({k: [2]})
        read
    ";
    assert_values(&[(source, "{k: [1]}")]);
}

#[test]
fn a_cell_rejects_a_function() {
    let message = "A mutable cell may not store a Function value";
    for value in [
        "fn -> 1",
        "print",
        "plus",
        "[1, fn -> 1]",
        "{a: {b: [plus]}}",
    ] {
        assert_raises(&[(&format!("mutable_cell({value})"), message)]);
        assert_raises(&[(&format!("mutable_cell().exchange({value})"), message)]);
    }
}

/// An Opaque payload with no drop glue: provably holds no Value.
#[derive(Debug)]
struct Plain(#[allow(dead_code)] u64);

impl FrostOpaque for Plain {
    fn type_name() -> &'static str {
        "Plain"
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }
}

/// An Opaque payload owning heap data, which a cell cannot see into.
#[derive(Debug)]
struct Owning(#[allow(dead_code)] Vec<u8>);

impl FrostOpaque for Owning {
    fn type_name() -> &'static str {
        "Owning"
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }
}

#[test]
fn a_cell_holds_an_opaque_that_holds_no_value() {
    let plain = Value::opaque(Plain(7));
    let special = Value::opaque(SpecialFloat::new(f64::INFINITY).unwrap());
    let source = r"
        def cell = mutable_cell([plain])
        def first = cell.get()
        cell.exchange({s: special})
        [first, cell.get()]
    ";
    let held = Script::new(source)
        .captures(&[("plain", plain.clone()), ("special", special.clone())])
        .run();
    assert_eq!(
        held,
        Value::array([Value::array([plain]), Value::map([("s", special)])])
    );
}

#[test]
fn a_cell_rejects_an_opaque_owning_heap_data() {
    let message =
        "A mutable cell may not store an Opaque Owning, which could refer back to the cell";
    for source in [
        "mutable_cell(owning)",
        "mutable_cell().exchange(owning)",
        "mutable_cell({a: [owning]})",
    ] {
        let raised = Script::new(source)
            .capture("owning", Value::opaque(Owning(vec![1])))
            .raises();
        assert_eq!(raised, message, "{source}");
    }
}

#[test]
fn a_rejected_exchange_leaves_the_cell_unchanged() {
    let source = r"
        def cell = mutable_cell('kept')
        def attempt = try_call(cell.exchange, [[fn -> 1]])
        [attempt.ok, cell.get()]
    ";
    assert_values(&[(source, "[false, 'kept']")]);
}

/// What a cell may not store is found at any depth, and the first of it found, left to
/// right and depth first, names the error.
#[test]
fn a_cell_rejects_the_first_forbidden_value_however_deep() {
    let function = "A mutable cell may not store a Function value";
    let opaque =
        "A mutable cell may not store an Opaque Owning, which could refer back to the cell";
    let cases = [
        ("mutable_cell([owning, fn -> 1])", opaque),
        ("mutable_cell([fn -> 1, owning])", function),
        ("mutable_cell([[[fn -> 1]], owning])", function),
        ("mutable_cell({a: [owning], b: fn -> 1})", opaque),
        ("mutable_cell(nest(fn -> 1, levels))", function),
        ("mutable_cell(nest(owning, levels))", opaque),
        ("mutable_cell().exchange(nest(fn -> 1, levels))", function),
    ];
    for (expression, message) in cases {
        let source = format!(
            r"
            defn nest(v, n) -> if n == 0: v else: nest([v, {{k: v}}], n - 1)
            {expression}
            "
        );
        let raised = crate::deep::on_a_small_stack(|| {
            Script::new(&source)
                .captures(&[
                    ("owning", Value::opaque(Owning(vec![1]))),
                    ("levels", Value::from(10_000)),
                ])
                .raises()
        });
        assert_eq!(raised, message, "{expression}");
    }
}

/// A cell holds a deeply nested value, and one sharing its contents, whose every
/// element in full would number 2^40: each container held in many places is looked
/// into once. How deep comes from the host as `levels`, and how many times doubled as
/// `doublings`, so that the values are built when the script runs rather than by the
/// compiler.
#[test]
fn a_cell_holds_a_deep_value_and_one_sharing_its_contents() {
    let source = r"
        defn nest(v, n) -> if n == 0: v else: nest([v, {k: v}], n - 1)
        defn doubled(v, n) -> if n == 0: v else: doubled([v, v], n - 1)
        def deep = nest(1, levels)
        def wide = doubled({k: 1}, doublings)
        def cell = mutable_cell(deep)
        def held_deep = cell.get() == deep
        cell.exchange(wide)
        [held_deep, cell.get() == wide]
    ";
    let tail = crate::deep::on_a_small_stack(|| {
        Script::new(source)
            .captures(&[
                ("levels", Value::from(10_000)),
                ("doublings", Value::from(40)),
            ])
            .run()
    });
    assert_eq!(tail, Value::from(vec![Value::from(true); 2]), "{source}");
}

#[test]
fn mutable_cell_takes_at_most_one_argument() {
    assert_arity("mutable_cell", "between 0 and 1", &[2, 3]);
}

#[test]
fn the_cell_functions_check_their_arity() {
    assert_arity_of("mutable_cell().get", "mutable_cell.get", 0, &[1, 2]);
    assert_arity_of("mutable_cell().take", "mutable_cell.take", 0, &[1, 2]);
    assert_arity_of(
        "mutable_cell().exchange",
        "mutable_cell.exchange",
        1,
        &[0, 2],
    );
}
