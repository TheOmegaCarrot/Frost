//! The `print` global, end to end: `print(value)` hands `value`, rendered as
//! `to_string` renders it, to the Vm's print sink, and returns Null.
//!
//! The harness captures what each run prints, and requires every optimization
//! permutation to print the same, so no optimization may drop, repeat, or reorder
//! a print.

mod common;

use common::{Script, raises, run};
use frost_runtime::Value;

fn printed(source: &str) -> Vec<String> {
    Script::new(source).printed()
}

#[test]
fn a_string_prints_as_its_text() {
    assert_eq!(printed(r#"print("hello")"#), ["hello"]);
    assert_eq!(printed(r#"print("")"#), [""]);
    assert_eq!(printed(r#"print("héllo, 世界")"#), ["héllo, 世界"]);
}

#[test]
fn any_value_prints_as_to_string_renders_it() {
    for value in [
        "null",
        "true",
        "-5",
        "1.5",
        "x'00ff'",
        r#"[1, "a", [null]]"#,
        r#"{a: "b"}"#,
        "plus",
        "fn -> 1",
    ] {
        let rendered = run(&format!("to_string({value})")).to_frost_string();
        assert_eq!(printed(&format!("print({value})")), [rendered], "{value}");
    }
}

#[test]
fn print_returns_null() {
    assert_eq!(run("print(1)"), Value::Null);
    assert_eq!(run("[print(1), print(2)]"), run("[null, null]"));
}

#[test]
fn each_print_is_one_text_without_a_line_terminator() {
    // A newline inside the text is passed through; none is added.
    let two_lines = r"b
c";
    let source = r#"
        print("a")
        print("b\nc")
    "#;
    assert_eq!(printed(source), ["a", two_lines]);
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

#[test]
fn print_takes_exactly_one_argument() {
    for source in ["print()", "print(1, 2)"] {
        let message = raises(source);
        assert!(message.contains("expects"), "{source:?}: {message}");
    }
    // An arity error prints nothing.
    assert_eq!(Script::new("print(1, 2)").printed(), Vec::<String>::new());
}
