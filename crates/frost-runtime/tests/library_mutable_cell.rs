//! The `mutable_cell` global, from Frost source: Frost's only built-in mutable
//! state.
//!
//! `mutable_cell(initial?)` returns a cell holding `initial`, or Null without
//! one: a Map of two Functions over the same state. `get()` returns the value
//! held, and `exchange(value)` holds `value` and returns the value it replaces.
//! A cell never holds a Function, even nested in an Array or Map.
//!
//! The harness runs every case under every optimization permutation, so each is
//! checked with and without folding the calls around a cell.

mod source;

use source::assertions::{Library, library_assertions};

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
fn a_cell_is_a_map_of_get_and_exchange() {
    assert_values(&[
        ("sorted(keys(mutable_cell()))", "['exchange', 'get']"),
        ("is_function(mutable_cell().get)", "true"),
        ("is_function(mutable_cell().exchange)", "true"),
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
        [cell.get(), holder.cell.get()]
    ";
    assert_values(&[(source, "[1, 1]")]);
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

#[test]
fn a_rejected_exchange_leaves_the_cell_unchanged() {
    let source = r"
        def cell = mutable_cell('kept')
        def attempt = try_call(cell.exchange, [[fn -> 1]])
        [attempt.ok, cell.get()]
    ";
    assert_values(&[(source, "[false, 'kept']")]);
}

#[test]
fn mutable_cell_takes_at_most_one_argument() {
    assert_arity("mutable_cell", "between 0 and 1", &[2, 3]);
}

#[test]
fn get_takes_no_arguments_and_exchange_exactly_one() {
    assert_arity_of("mutable_cell().get", "mutable_cell.get", 0, &[1, 2]);
    assert_arity_of(
        "mutable_cell().exchange",
        "mutable_cell.exchange",
        1,
        &[0, 2],
    );
}
