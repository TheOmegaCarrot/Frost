//! Effects under compilation, observed through `print`, which stands for any
//! impure global: a script's effects happen in evaluation order, exactly when
//! the code making them runs.
//!
//! The harness captures what each run prints, and requires every optimization
//! permutation to print the same, so no optimization may drop, repeat, or reorder
//! a print.

mod script;

use script::Script;

fn printed(source: &str) -> Vec<String> {
    Script::new(source).printed()
}

#[test]
fn prints_happen_in_evaluation_order() {
    let sequential = r"
        print(1)
        print(2)
        print(3)
    ";
    assert_eq!(printed(sequential), ["1", "2", "3"]);
    assert_eq!(printed("[print(1), print(2)]"), ["1", "2"]);
    assert_eq!(printed("plus(print(1) or 1, print(2) or 2)"), ["1", "2"]);
    let in_transform = r"
        transform([1, 2, 3], fn x -> {
            print(x)
            x
        })
    ";
    assert_eq!(printed(in_transform), ["1", "2", "3"]);
    let recursive = r"
        defn count(n) -> if n == 0: 0 else: do {
            print(n)
            count(n - 1)
        }
        count(3)
    ";
    assert_eq!(printed(recursive), ["3", "2", "1"]);
}

#[test]
fn only_the_prints_that_run_happen() {
    assert_eq!(
        printed(r#"if false: print("no") else: 1"#),
        Vec::<String>::new()
    );
    assert_eq!(printed(r#"true or print("no")"#), Vec::<String>::new());
    let unused_function = r#"
        def f = fn -> print("no")
        1
    "#;
    assert_eq!(printed(unused_function), Vec::<String>::new());
    assert_eq!(
        printed(r#"match 2 { 1 => print("one"), 2 => print("two") }"#),
        ["two"]
    );
}

#[test]
fn a_bound_print_happens_once() {
    let bound_value = r"
        def x = print(1)
        [x, x]
    ";
    assert_eq!(printed(bound_value), ["1"]);
    let bound_function = r"
        def f = fn -> print(1)
        f()
        f()
    ";
    assert_eq!(printed(bound_function), ["1", "1"], "once per call");
}

#[test]
fn prints_before_an_error_still_happen() {
    let script = Script::new(
        r#"
        print("before")
        error("boom")
        print("after")
        "#,
    );
    assert_eq!(script.printed(), ["before"]);
    assert!(script.raises().contains("boom"));
}
