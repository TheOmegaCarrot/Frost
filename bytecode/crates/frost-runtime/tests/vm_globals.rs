//! Tests for the currently-implemented globals: the `is_*` type predicates, the
//! `type`/`to_int`/`to_float` conversions, and the operators. Each is driven the
//! way real code reaches a global -- `LoadGlobal` + `Call` -- so the test exercises
//! the full native-call path, not just the constructor.
//!
//! (`call` and `try_call` are implemented too, but already covered in
//! `vm_call.rs` and `vm_errors.rs`.)

use std::collections::BTreeMap;
use std::sync::Arc;

use frost_runtime::{
    Arity, Bytecode, CompiledFunction, FrostArray, FrostError, FrostFloat, GlobalSet, MapKey,
    NameEntry, NativeFunction, Value, Vm,
};

use Bytecode::*;

// ============================================================
// Harness: invoke a global by name with the given argument values
// ============================================================

/// Call global `name` with `args`, returning its result (or the raised error).
/// The args are seated as captures so any `Value` can be passed; the body loads
/// them and `Call`s the global, exactly as compiled code would.
fn run_global(name: &str, args: Vec<Value>) -> Result<Value, FrostError> {
    let slot = GlobalSet::defaults()
        .index_of(name)
        .unwrap_or_else(|| panic!("`{name}` is not a predefined global"));
    let argc = args.len();

    let mut code = vec![Pop, LoadGlobal(slot)];
    code.extend((0..argc).map(LoadLocal));
    code.push(Call(argc));

    let name_table = (0..argc)
        .map(|i| NameEntry {
            name: format!("a{i}"),
            exported: false,
        })
        .collect();
    let main = Arc::new(CompiledFunction {
        name: "main".to_string(),
        code,
        child_fns: Vec::new(),
        constants: Vec::new(),
        name_table,
        num_captures: argc,
        arity: Arity::Exact(0),
    });
    let captures: BTreeMap<String, Value> = args
        .into_iter()
        .enumerate()
        .map(|(i, v)| (format!("a{i}"), v))
        .collect();
    Vm::new(main.close(captures).unwrap())
        .unwrap()
        .run()
        .map(|r| r.tail().clone())
}

/// `run_global`, unwrapping the success value.
fn g(name: &str, args: Vec<Value>) -> Value {
    run_global(name, args).unwrap()
}

fn float(f: f64) -> Value {
    Value::Float(FrostFloat::new(f).unwrap())
}

fn arr(elems: Vec<Value>) -> Value {
    Value::Array(FrostArray::from(elems))
}

fn fmap(pairs: Vec<(&str, Value)>) -> Value {
    Value::Map(
        pairs
            .into_iter()
            .map(|(k, v)| (MapKey::String(Arc::from(k.as_bytes())), v))
            .collect(),
    )
}

fn func_value() -> Value {
    Value::NativeFunction(Arc::new(NativeFunction::new(
        |_, _: &mut [Value]| Ok(Value::Null),
        "f",
        Arity::Exact(0),
    )))
}

// ============================================================
// Type predicates
// ============================================================

#[test]
fn type_predicates() {
    let null = Value::Null;
    let boolean = Value::Bool(true);
    let int = Value::Int(1);
    let flt = float(1.5);
    let string = Value::from("x");
    let array = arr(vec![Value::Int(1)]);
    let map = fmap(vec![("a", Value::Int(1))]);
    let function = func_value();

    let cases: &[(&str, &Value, bool)] = &[
        ("is_null", &null, true),
        ("is_null", &int, false),
        ("is_bool", &boolean, true),
        ("is_bool", &int, false),
        ("is_int", &int, true),
        ("is_int", &flt, false),
        ("is_float", &flt, true),
        ("is_float", &int, false),
        ("is_string", &string, true),
        ("is_string", &int, false),
        ("is_array", &array, true),
        ("is_array", &map, false),
        ("is_map", &map, true),
        ("is_map", &array, false),
        ("is_function", &function, true),
        ("is_function", &int, false),
        ("is_nonnull", &int, true),
        ("is_nonnull", &null, false),
        ("is_numeric", &int, true),
        ("is_numeric", &flt, true),
        ("is_numeric", &string, false),
        ("is_primitive", &null, true),
        ("is_primitive", &boolean, true),
        ("is_primitive", &int, true),
        ("is_primitive", &flt, true),
        ("is_primitive", &string, true),
        ("is_primitive", &array, false),
        ("is_primitive", &function, false),
        ("is_structured", &array, true),
        ("is_structured", &map, true),
        ("is_structured", &int, false),
        ("is_structured", &string, false),
    ];

    for &(global, value, expected) in cases {
        assert_eq!(
            g(global, vec![value.clone()]),
            Value::Bool(expected),
            "{global} on {value:?}"
        );
    }
}

// ============================================================
// Conversions: type, to_int, to_float
// ============================================================

#[test]
fn type_names_the_value_type() {
    assert_eq!(g("type", vec![Value::Null]), Value::from("Null"));
    assert_eq!(g("type", vec![Value::Bool(true)]), Value::from("Bool"));
    assert_eq!(g("type", vec![Value::Int(1)]), Value::from("Int"));
    assert_eq!(g("type", vec![float(1.5)]), Value::from("Float"));
    assert_eq!(g("type", vec![Value::from("x")]), Value::from("String"));
    assert_eq!(g("type", vec![arr(vec![])]), Value::from("Array"));
    assert_eq!(g("type", vec![fmap(vec![])]), Value::from("Map"));
    assert_eq!(g("type", vec![func_value()]), Value::from("Function"));
}

#[test]
fn to_int_converts_or_nulls() {
    assert_eq!(g("to_int", vec![Value::Int(5)]), Value::Int(5));
    assert_eq!(g("to_int", vec![float(1.9)]), Value::Int(1)); // truncates toward zero
    assert_eq!(g("to_int", vec![Value::from("42")]), Value::Int(42));
    assert_eq!(g("to_int", vec![Value::from("nope")]), Value::Null);
    assert_eq!(g("to_int", vec![Value::Null]), Value::Null);
    assert_eq!(g("to_int", vec![Value::Bool(true)]), Value::Null);
}

#[test]
fn to_float_converts_or_nulls() {
    assert_eq!(g("to_float", vec![float(1.5)]), float(1.5));
    assert_eq!(g("to_float", vec![Value::Int(2)]), float(2.0));
    assert_eq!(g("to_float", vec![Value::from("3.5")]), float(3.5));
    assert_eq!(g("to_float", vec![Value::from("nope")]), Value::Null);
    assert_eq!(g("to_float", vec![Value::Null]), Value::Null);
}

// ============================================================
// Arithmetic operators
// ============================================================

#[test]
fn plus_adds_and_concatenates() {
    assert_eq!(g("plus", vec![Value::Int(1), Value::Int(2)]), Value::Int(3));
    assert_eq!(g("plus", vec![float(1.5), float(2.5)]), float(4.0));
    assert_eq!(g("plus", vec![Value::Int(1), float(2.0)]), float(3.0)); // mixed promotes
    assert_eq!(
        g("plus", vec![Value::from("a"), Value::from("b")]),
        Value::from("ab")
    );
}

#[test]
fn plus_concatenates_arrays() {
    assert_eq!(
        g(
            "plus",
            vec![
                arr(vec![Value::Int(1), Value::Int(2)]),
                arr(vec![Value::Int(3)]),
            ],
        ),
        arr(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn plus_merges_maps_with_rhs_winning() {
    assert_eq!(
        g(
            "plus",
            vec![
                fmap(vec![("a", Value::Int(1))]),
                fmap(vec![("b", Value::Int(2))]),
            ],
        ),
        fmap(vec![("a", Value::Int(1)), ("b", Value::Int(2))])
    );
    // Key collision: the right-hand value wins, matching `+`.
    assert_eq!(
        g(
            "plus",
            vec![
                fmap(vec![("a", Value::Int(1))]),
                fmap(vec![("a", Value::Int(2))]),
            ],
        ),
        fmap(vec![("a", Value::Int(2))])
    );
}

#[test]
fn plus_type_mismatch_errors() {
    assert!(run_global("plus", vec![Value::Int(1), Value::from("x")]).is_err());
}

#[test]
fn minus_times_divide_mod() {
    assert_eq!(g("minus", vec![Value::Int(5), Value::Int(3)]), Value::Int(2));
    assert_eq!(g("times", vec![Value::Int(4), Value::Int(3)]), Value::Int(12));
    assert_eq!(g("divide", vec![Value::Int(10), Value::Int(2)]), Value::Int(5)); // integer division
    assert_eq!(g("mod", vec![Value::Int(10), Value::Int(3)]), Value::Int(1));
}

#[test]
fn divide_and_mod_by_zero_error() {
    assert!(run_global("divide", vec![Value::Int(1), Value::Int(0)]).is_err());
    assert!(run_global("mod", vec![Value::Int(1), Value::Int(0)]).is_err());
}

// ============================================================
// Comparison and equality operators
// ============================================================

#[test]
fn equal_and_not_equal() {
    assert_eq!(
        g("equal", vec![Value::Int(1), Value::Int(1)]),
        Value::Bool(true)
    );
    assert_eq!(
        g("equal", vec![Value::Int(1), Value::Int(2)]),
        Value::Bool(false)
    );
    // No cross-type numeric equality: 1 != 1.0.
    assert_eq!(
        g("equal", vec![Value::Int(1), float(1.0)]),
        Value::Bool(false)
    );
    assert_eq!(
        g("not_equal", vec![Value::Int(1), Value::Int(2)]),
        Value::Bool(true)
    );
    assert_eq!(
        g("not_equal", vec![Value::Int(1), Value::Int(1)]),
        Value::Bool(false)
    );
}

#[test]
fn ordering_comparisons() {
    let t = Value::Bool(true);
    let f = Value::Bool(false);

    assert_eq!(g("less_than", vec![Value::Int(1), Value::Int(2)]), t);
    assert_eq!(g("less_than", vec![Value::Int(2), Value::Int(1)]), f);
    assert_eq!(g("less_than", vec![Value::Int(1), Value::Int(1)]), f);

    assert_eq!(g("less_than_or_equal", vec![Value::Int(1), Value::Int(1)]), t);
    assert_eq!(g("less_than_or_equal", vec![Value::Int(2), Value::Int(1)]), f);

    assert_eq!(g("greater_than", vec![Value::Int(2), Value::Int(1)]), t);
    assert_eq!(g("greater_than", vec![Value::Int(1), Value::Int(2)]), f);

    assert_eq!(
        g("greater_than_or_equal", vec![Value::Int(1), Value::Int(1)]),
        t
    );
    assert_eq!(
        g("greater_than_or_equal", vec![Value::Int(1), Value::Int(2)]),
        f
    );

    // `<` also orders Floats and mixed numerics.
    assert_eq!(g("less_than", vec![Value::Int(1), float(1.5)]), Value::Bool(true));
}

#[test]
fn comparison_across_unorderable_types_errors() {
    assert!(run_global("less_than", vec![Value::from("a"), Value::Int(1)]).is_err());
}

// ============================================================
// Arity (every implemented global rejects the wrong argument count)
// ============================================================

#[test]
fn globals_enforce_arity() {
    let unary = [
        "is_null",
        "is_int",
        "is_float",
        "is_bool",
        "is_string",
        "is_array",
        "is_map",
        "is_function",
        "is_nonnull",
        "is_numeric",
        "is_primitive",
        "is_structured",
        "type",
        "to_int",
        "to_float",
    ];
    for name in unary {
        assert!(run_global(name, vec![]).is_err(), "{name} with 0 args");
        assert!(
            run_global(name, vec![Value::Int(1), Value::Int(2)]).is_err(),
            "{name} with 2 args"
        );
    }

    let binary = [
        "plus",
        "minus",
        "times",
        "divide",
        "mod",
        "equal",
        "not_equal",
        "less_than",
        "less_than_or_equal",
        "greater_than",
        "greater_than_or_equal",
    ];
    for name in binary {
        assert!(
            run_global(name, vec![Value::Int(1)]).is_err(),
            "{name} with 1 arg"
        );
        assert!(
            run_global(name, vec![Value::Int(1), Value::Int(2), Value::Int(3)]).is_err(),
            "{name} with 3 args"
        );
    }
}
