//! Literal compilation, end to end: compile full source, run it on the VM, and
//! assert the tail value. Behavioral by design; a literal's job is to evaluate
//! to the right value, not to emit a particular opcode (which optimization is
//! free to change).

mod common;

use common::{compile_errors, run};
use frost_runtime::Value;

#[test]
fn int_literal() {
    assert_eq!(run("42"), Value::Int(42));
    assert_eq!(run("0"), Value::Int(0));
    assert_eq!(run("9223372036854775807"), Value::Int(i64::MAX));
}

#[test]
fn bool_literals() {
    assert_eq!(run("true"), Value::Bool(true));
    assert_eq!(run("false"), Value::Bool(false));
}

#[test]
fn null_literal() {
    assert_eq!(run("null"), Value::Null);
}

#[test]
fn float_literal() {
    assert_eq!(run("3.5"), Value::try_from(3.5).unwrap());
}

#[test]
fn whole_number_float_is_distinct_from_int() {
    // Frost has no cross-type numeric equality: `2` and `2.0` are different
    // values, and the compiler must keep the literal's type.
    assert_eq!(run("2.0"), Value::try_from(2.0).unwrap());
    assert_eq!(run("2"), Value::Int(2));
    assert_ne!(run("2"), run("2.0"), "Int 2 and Float 2.0 are not equal");
}

#[test]
fn string_literal() {
    assert_eq!(run(r#""hi""#), Value::from("hi"));
    assert_eq!(run(r#""""#), Value::from(""), "empty string");
}

#[test]
fn bytes_literal() {
    // `x'..'` carries hex-pair octets.
    assert_eq!(run("x'6869'"), Value::from(vec![0x68u8, 0x69]));
    assert_eq!(run("x''"), Value::from(Vec::<u8>::new()), "empty bytes");
    assert_eq!(
        run("x'00ff'"),
        Value::from(vec![0x00u8, 0xff]),
        "full octet range, including bytes no String could hold"
    );
}

#[test]
fn float_that_overflows_to_infinity_is_a_compile_error() {
    // A finite-looking literal whose magnitude overflows f64 is rejected at
    // compile time by the Float validator (the reachable path, since `NaN`
    // and `Infinity` are not writable literals).
    let rendered = compile_errors("1e400").render_plain();
    assert!(
        rendered.contains("Float"),
        "the diagnostic names the Float validator that rejected it:\n{rendered}"
    );
    assert!(
        rendered.contains("1e400"),
        "the offending literal is shown:\n{rendered}"
    );
}
