//! Tests for [`Bytecode::MatchPattern`].
//!
//! Most tests reach [`run`]'s program through the `assert_*` helpers: it
//! defines each local a pattern may bind as [`unset`] first, then matches a
//! value and reads every such local back, so a local the match left alone reads
//! as [`unset`]. Whether a match leaves anything holding a part is asked of the
//! `unshared` probe instead, and matching in a called frame is checked by a
//! program of its own.

use std::sync::Arc;

use crate::common::pattern::{
    array_as, at_least, bind, discard, either, exact, input_key, int, key, map, pin, typed, value,
};
use crate::common::{self, Pop, entry, func_with_captures, global_slot, with_patterns};
use crate::unshared::unshared;

use frostlang::bytecode::{
    Bytecode, CompiledFunction, CompiledPattern, FormatVersion, PatternComparand, PatternInput,
};
use frostlang::{Arity, FrostError, FrostFloat, FrostMap, FrostType, MapKey, Value, Vm};

use Bytecode::*;

// The locals a pattern may bind, in slot order.
const A: u32 = 0;
const B: u32 = 1;
const C: u32 = 2;
/// A local holding [`Setup::input`], defined before the match, for a pattern
/// to take as an input.
const INPUT: u32 = 3;

/// What every local a pattern may bind holds before the match.
fn unset() -> Value {
    Value::from("unset")
}

/// What a program holds besides its pattern.
#[derive(Clone)]
struct Setup {
    constants: Vec<Value>,
    keys: Vec<MapKey>,
    /// The value of the [`INPUT`] local.
    input: Value,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            constants: Vec::new(),
            keys: Vec::new(),
            input: Value::Null,
        }
    }
}

impl Setup {
    fn with_keys(keys: &[&str]) -> Self {
        Self {
            keys: keys.iter().map(|&key| MapKey::from(key)).collect(),
            ..Self::default()
        }
    }

    fn with_input(self, input: Value) -> Self {
        Self { input, ..self }
    }
}

/// What matching a value against a pattern left.
#[derive(Debug, PartialEq)]
struct Outcome {
    matched: bool,
    /// The matched value, as left beneath the result.
    tested: Value,
    /// The locals [`A`], [`B`], and [`C`].
    locals: [Value; 3],
}

/// Match the value `tested` leaves against `pattern`, as pattern 0 of a
/// function holding `setup`, after [`A`], [`B`], and [`C`] are defined as
/// [`unset`].
fn run(
    pattern: CompiledPattern,
    setup: Setup,
    tested: Vec<Bytecode>,
) -> Result<Outcome, FrostError> {
    let Setup {
        mut constants,
        keys,
        input,
    } = setup;
    let unset_at = constants.len() as u64;
    constants.extend([unset(), input]);
    let code = [
        vec![
            Pop, // the closure value the runner pushes
            LoadConst(unset_at),
            DefLocal(A.into()),
            LoadConst(unset_at),
            DefLocal(B.into()),
            LoadConst(unset_at),
            DefLocal(C.into()),
            LoadConst(unset_at + 1),
            DefLocal(INPUT.into()),
        ],
        tested,
        vec![
            MatchPattern(0),
            MakeArray(2),
            LoadLocal(A.into()),
            LoadLocal(B.into()),
            LoadLocal(C.into()),
            MakeArray(3),
            MakeArray(2),
        ],
    ]
    .concat();
    let program = Arc::new(CompiledFunction {
        version: FormatVersion,
        name: "<match-pattern>".to_string(),
        origin: None,
        code,
        child_fns: Vec::new(),
        constants,
        key_constants: keys,
        patterns: vec![pattern],
        name_table: ["a", "b", "c", "input"]
            .map(|name| entry(name, false))
            .to_vec(),
        num_captures: 0,
        arity: Arity::Exact(0),
        debug_info: None,
    });
    let closure = program.assert_well_formed().into_closure().unwrap();
    let result = Vm::factory()
        .build(closure)
        .run()
        .map_err(frostlang::RunError::into_error)?;
    let [stack, locals] = parts(result.tail());
    let [tested, matched] = parts(&stack);
    let Value::Bool(matched) = matched else {
        panic!("`MatchPattern` pushed {matched:?}, not a Bool");
    };
    Ok(Outcome {
        matched,
        tested,
        locals: parts(&locals),
    })
}

/// The elements of `array`, which has exactly `N`.
fn parts<const N: usize>(array: &Value) -> [Value; N] {
    let elements: Vec<Value> = array
        .as_array()
        .expect("an Array")
        .iter()
        .cloned()
        .collect();
    elements.try_into().expect("as many elements as expected")
}

/// Match `tested` against `pattern`, in a function holding `setup`.
fn try_match(
    pattern: &CompiledPattern,
    setup: Setup,
    tested: &Value,
) -> Result<Outcome, FrostError> {
    let mut setup = setup;
    let at = setup.constants.len() as u64;
    setup.constants.push(tested.clone());
    run(pattern.clone(), setup, vec![LoadConst(at)])
}

/// Match `tested` against `pattern`, in a function holding `setup`, asserting
/// it raises no error and is kept beneath the result.
fn matched_locals(pattern: &CompiledPattern, setup: Setup, tested: &Value) -> (bool, [Value; 3]) {
    let outcome = try_match(pattern, setup, tested)
        .unwrap_or_else(|error| panic!("{pattern:?} against {tested:?} raised: {error}"));
    assert_eq!(
        &outcome.tested, tested,
        "{pattern:?} keeps the matched value beneath the result"
    );
    (outcome.matched, outcome.locals)
}

/// Assert `tested` matches `pattern`, in a function holding `setup`, binding
/// [`A`], [`B`], and [`C`] as `locals` says; `None` leaves one [`unset`].
fn assert_binds_with(
    pattern: &CompiledPattern,
    setup: Setup,
    tested: &Value,
    locals: [Option<Value>; 3],
) {
    let expected = locals.map(|local| local.unwrap_or_else(unset));
    assert_eq!(
        matched_locals(pattern, setup, tested),
        (true, expected),
        "{pattern:?} against {tested:?}"
    );
}

/// [`assert_binds_with`], in a function holding the default [`Setup`].
fn assert_binds(pattern: &CompiledPattern, tested: &Value, locals: [Option<Value>; 3]) {
    assert_binds_with(pattern, Setup::default(), tested, locals);
}

/// Assert `tested` does not match `pattern`, in a function holding `setup`,
/// leaving every local [`unset`].
fn assert_mismatch_with(pattern: &CompiledPattern, setup: Setup, tested: &Value) {
    assert_eq!(
        matched_locals(pattern, setup, tested),
        (false, [unset(), unset(), unset()]),
        "{pattern:?} against {tested:?}"
    );
}

/// [`assert_mismatch_with`], in a function holding the default [`Setup`].
fn assert_mismatch(pattern: &CompiledPattern, tested: &Value) {
    assert_mismatch_with(pattern, Setup::default(), tested);
}

/// Assert matching `tested` against `pattern`, in a function holding `setup`,
/// raises `message`.
fn assert_raises_with(pattern: &CompiledPattern, setup: Setup, tested: &Value, message: &str) {
    let error = try_match(pattern, setup, tested)
        .expect_err(&format!("{pattern:?} against {tested:?} raises"));
    assert_eq!(error.message(), message, "{pattern:?} against {tested:?}");
}

/// `{[input]: _}`: looks up the [`INPUT`] local as a key.
fn input_key_lookup() -> CompiledPattern {
    map([(input_key(PatternInput::Local(INPUT)), discard())], None)
}

fn ints(values: &[i64]) -> Value {
    Value::from(
        values
            .iter()
            .map(|&value| Value::from(value))
            .collect::<Vec<_>>(),
    )
}

fn float(value: f64) -> FrostFloat {
    FrostFloat::new(value).unwrap()
}

fn one(value: impl Into<Value>) -> Option<Value> {
    Some(value.into())
}

// ---- Bindings ----

#[test]
fn a_binding_matches_any_value_and_binds_it() {
    for tested in [
        Value::Null,
        Value::from(1),
        Value::from("s"),
        ints(&[1, 2]),
        Value::map([("k", Value::from(1))]),
    ] {
        assert_binds(&bind(A), &tested, [Some(tested.clone()), None, None]);
    }
}

#[test]
fn a_discard_matches_any_value_and_binds_nothing() {
    for tested in [Value::Null, Value::from(1), ints(&[1, 2])] {
        assert_binds(&discard(), &tested, [None, None, None]);
    }
}

#[test]
fn a_typed_binding_matches_only_a_value_of_its_types() {
    let numeric = typed(FrostType::INT | FrostType::FLOAT, Some(A));
    for tested in [Value::from(1), Value::Float(float(1.5))] {
        assert_binds(&numeric, &tested, [Some(tested.clone()), None, None]);
    }
    for tested in [Value::Null, Value::from("1"), ints(&[1])] {
        assert_mismatch(&numeric, &tested);
    }
}

#[test]
fn a_typed_discard_tests_the_type_and_binds_nothing() {
    let string = typed(FrostType::STRING, None);
    assert_binds(&string, &Value::from("s"), [None, None, None]);
    assert_mismatch(&string, &Value::from(1));
}

// ---- Values ----

#[test]
fn an_immediate_comparand_matches_only_an_equal_value() {
    let cases = [
        (PatternComparand::Null, Value::Null, Value::from(false)),
        (
            PatternComparand::Bool(true),
            Value::from(true),
            Value::from(1),
        ),
        (PatternComparand::Int(1), Value::from(1), Value::from(2)),
        (
            PatternComparand::Float(float(1.5)),
            Value::Float(float(1.5)),
            Value::Float(float(2.5)),
        ),
    ];
    for (comparand, equal, unequal) in cases {
        let pattern = value(comparand);
        assert_binds(&pattern, &equal, [None, None, None]);
        assert_mismatch(&pattern, &unequal);
    }
}

#[test]
fn a_comparand_compares_by_value_equality_without_numeric_coercion() {
    // `1 == 1.0` and `0.0 == -0.0` are both false.
    assert_mismatch(&int(1), &Value::Float(float(1.0)));
    let zero = value(PatternComparand::Float(float(0.0)));
    assert_mismatch(&zero, &Value::Float(float(-0.0)));
    assert_mismatch(&zero, &Value::from(0));
}

#[test]
fn a_constant_comparand_matches_only_an_equal_value() {
    let pattern = value(PatternComparand::Constant(0));
    let setup = Setup {
        constants: vec![ints(&[1, 2])],
        ..Setup::default()
    };
    assert_binds_with(&pattern, setup.clone(), &ints(&[1, 2]), [None, None, None]);
    assert_mismatch_with(&pattern, setup, &ints(&[1, 3]));
}

#[test]
fn a_local_comparand_matches_only_a_value_equal_to_the_local() {
    let pattern = pin(PatternInput::Local(INPUT));
    let setup = Setup::default().with_input(Value::from("x"));
    assert_binds_with(
        &pattern,
        setup.clone(),
        &Value::from("x"),
        [None, None, None],
    );
    assert_mismatch_with(&pattern, setup, &Value::from("y"));
}

#[test]
fn a_local_comparand_reads_the_local_as_it_was_before_the_match() {
    // `[a, (local a)]`: the match defines `a` only once it is done, so
    // `(local a)` reads what `a` held before.
    let pattern = exact([bind(A), pin(PatternInput::Local(A))]);
    assert_binds(
        &pattern,
        &Value::array([Value::from(1), unset()]),
        [one(1), None, None],
    );
    assert_mismatch(&pattern, &ints(&[1, 1]));
}

#[test]
fn a_global_comparand_matches_only_that_global() {
    let type_global = global_slot("type");
    let pattern = exact([pin(PatternInput::Global(type_global as u32))]);
    // `[type]` matches, `[len]` does not.
    let matched = |global| {
        run(
            pattern.clone(),
            Setup::default(),
            vec![LoadGlobal(global), MakeArray(1)],
        )
        .unwrap()
        .matched
    };
    assert!(matched(type_global), "the same global");
    assert!(!matched(global_slot("len")), "another global");
}

#[test]
fn an_earlier_binding_comparand_matches_only_a_value_equal_to_it() {
    // `[a, (a)]`
    let pattern = exact([bind(A), pin(PatternInput::EarlierBinding(A))]);
    assert_binds(&pattern, &ints(&[1, 1]), [one(1), None, None]);
    assert_mismatch(&pattern, &ints(&[1, 2]));
}

#[test]
fn an_earlier_rest_binding_compares_as_the_array_it_binds() {
    // `{xs: [_, ...a], ys: (a)}`
    let pattern = map(
        [
            (key(0), at_least([discard()], Some(A))),
            (key(1), pin(PatternInput::EarlierBinding(A))),
        ],
        None,
    );
    let tested = |ys: Value| Value::map([("xs", ints(&[0, 1, 2])), ("ys", ys)]);
    let setup = Setup::with_keys(&["xs", "ys"]);
    assert_binds_with(
        &pattern,
        setup.clone(),
        &tested(ints(&[1, 2])),
        [Some(ints(&[1, 2])), None, None],
    );
    assert_mismatch_with(&pattern, setup, &tested(ints(&[1])));
}

#[test]
fn an_earlier_whole_array_binding_compares_as_the_array() {
    // `[[..._] as a, (a)]`
    let pattern = exact([
        array_as(at_least([], None), A),
        pin(PatternInput::EarlierBinding(A)),
    ]);
    assert_binds(
        &pattern,
        &Value::array([ints(&[1, 2]), ints(&[1, 2])]),
        [Some(ints(&[1, 2])), None, None],
    );
    assert_mismatch(&pattern, &Value::array([ints(&[1, 2]), ints(&[1])]));
}

#[test]
fn an_earlier_whole_map_binding_compares_as_the_map() {
    // `[{} as a, (a)]`
    let pattern = exact([map([], Some(A)), pin(PatternInput::EarlierBinding(A))]);
    let one_map = Value::map([("k", Value::from(1))]);
    let other_map = Value::map([("k", Value::from(2))]);
    assert_binds(
        &pattern,
        &Value::array([one_map.clone(), one_map.clone()]),
        [Some(one_map.clone()), None, None],
    );
    assert_mismatch(&pattern, &Value::array([one_map, other_map]));
}

// ---- Arrays ----

#[test]
fn an_exact_array_pattern_matches_only_an_array_of_its_length() {
    let pattern = exact([bind(A), bind(B)]);
    assert_binds(&pattern, &ints(&[1, 2]), [one(1), one(2), None]);
    for tested in [
        ints(&[]),
        ints(&[1]),
        ints(&[1, 2, 3]),
        Value::from("ab"),
        Value::map([("a", Value::from(1))]),
    ] {
        assert_mismatch(&pattern, &tested);
    }
}

#[test]
fn an_empty_exact_array_pattern_matches_only_an_empty_array() {
    assert_binds(&exact([]), &ints(&[]), [None, None, None]);
    assert_mismatch(&exact([]), &ints(&[1]));
}

#[test]
fn an_array_pattern_with_a_rest_binds_the_remaining_elements() {
    let pattern = at_least([bind(A)], Some(B));
    assert_binds(
        &pattern,
        &ints(&[1, 2, 3]),
        [one(1), Some(ints(&[2, 3])), None],
    );
    assert_binds(&pattern, &ints(&[1]), [one(1), Some(ints(&[])), None]);
    assert_mismatch(&pattern, &ints(&[]));
    assert_mismatch(&pattern, &Value::Null);
}

#[test]
fn an_array_pattern_with_a_discarded_rest_binds_only_its_elements() {
    assert_binds(
        &at_least([bind(A)], None),
        &ints(&[1, 2, 3]),
        [one(1), None, None],
    );
}

#[test]
fn an_array_pattern_of_only_a_rest_matches_any_array() {
    for rest in [Some(A), None] {
        let pattern = at_least([], rest);
        for tested in [ints(&[]), ints(&[1, 2])] {
            let bound = rest.map(|_| tested.clone());
            assert_binds(&pattern, &tested, [bound, None, None]);
        }
        for tested in [Value::from("ab"), Value::from(FrostMap::default())] {
            assert_mismatch(&pattern, &tested);
        }
    }
}

#[test]
fn an_element_mismatch_after_a_binding_binds_nothing() {
    // `[a, 2, c]`: `a` is bound before `2` fails.
    assert_mismatch(&exact([bind(A), int(2), bind(C)]), &ints(&[1, 3, 4]));
    // `[[_, ...a], 4]`: the rest `a` is bound before `4` fails.
    assert_mismatch(
        &exact([at_least([discard()], Some(A)), int(4)]),
        &Value::array([ints(&[1, 2, 3]), Value::from(3)]),
    );
}

#[test]
fn an_array_pattern_binds_the_whole_array() {
    // `[a, b] as c`
    let pattern = array_as(exact([bind(A), bind(B)]), C);
    assert_binds(
        &pattern,
        &ints(&[1, 2]),
        [one(1), one(2), Some(ints(&[1, 2]))],
    );
    for tested in [ints(&[1]), ints(&[1, 2, 3]), Value::Null] {
        assert_mismatch(&pattern, &tested);
    }
}

#[test]
fn an_array_pattern_binds_its_rest_and_the_whole_array() {
    // `[a, ...b] as c`
    assert_binds(
        &array_as(at_least([bind(A)], Some(B)), C),
        &ints(&[1, 2, 3]),
        [one(1), Some(ints(&[2, 3])), Some(ints(&[1, 2, 3]))],
    );
    // `[..._] as a`
    let pattern = array_as(at_least([], None), A);
    for tested in [ints(&[]), ints(&[1, 2])] {
        assert_binds(&pattern, &tested, [Some(tested.clone()), None, None]);
    }
    assert_mismatch(&pattern, &Value::map([("k", Value::from(1))]));
}

#[test]
fn an_element_mismatch_binds_no_whole_array() {
    // `[a, 2] as b`
    assert_mismatch(&array_as(exact([bind(A), int(2)]), B), &ints(&[1, 3]));
    // `[[] as a, 4]`: the whole Array `a` is bound before `4` fails.
    assert_mismatch(
        &exact([array_as(exact([]), A), int(4)]),
        &Value::array([ints(&[]), Value::from(3)]),
    );
}

#[test]
fn nested_array_patterns_bind_at_every_depth() {
    // `[a, [b, ...c]]`
    let pattern = exact([bind(A), at_least([bind(B)], Some(C))]);
    assert_binds(
        &pattern,
        &Value::array([Value::from(1), ints(&[2, 3, 4])]),
        [one(1), one(2), Some(ints(&[3, 4]))],
    );
}

// ---- Maps ----

#[test]
fn a_map_pattern_matches_only_a_map_with_every_key() {
    // `{k: a, j: b}`
    let pattern = map([(key(0), bind(A)), (key(1), bind(B))], None);
    let setup = Setup::with_keys(&["k", "j"]);
    let full = Value::map([
        ("k", Value::from(1)),
        ("j", Value::from(2)),
        ("x", Value::from(3)),
    ]);
    assert_binds_with(&pattern, setup.clone(), &full, [one(1), one(2), None]);
    let missing_j = Value::map([("k", Value::from(1))]);
    for tested in [missing_j, ints(&[1, 2]), Value::Null] {
        assert_mismatch_with(&pattern, setup.clone(), &tested);
    }
}

#[test]
fn a_map_pattern_entry_mismatch_binds_nothing() {
    // `{k: a, j: 2} as b`
    assert_mismatch_with(
        &map([(key(0), bind(A)), (key(1), int(2))], Some(B)),
        Setup::with_keys(&["k", "j"]),
        &Value::map([("k", Value::from(1)), ("j", Value::from(3))]),
    );
    // `[{} as a, 4]`: the whole Map `a` is bound before `4` fails.
    assert_mismatch(
        &exact([map([], Some(A)), int(4)]),
        &Value::array([Value::map([("k", Value::from(1))]), Value::from(3)]),
    );
}

#[test]
fn a_map_pattern_binds_the_whole_map() {
    // `{k: a} as b`
    let tested = Value::map([("k", Value::from(1))]);
    assert_binds_with(
        &map([(key(0), bind(A))], Some(B)),
        Setup::with_keys(&["k"]),
        &tested,
        [one(1), Some(tested.clone()), None],
    );
}

#[test]
fn an_empty_map_pattern_matches_any_map() {
    let pattern = map([], Some(A));
    let tested = Value::map([("k", Value::from(1))]);
    assert_binds(&pattern, &tested, [Some(tested.clone()), None, None]);
    assert_mismatch(&pattern, &ints(&[]));
}

#[test]
fn a_key_present_with_a_null_value_is_present() {
    let setup = Setup::with_keys(&["k"]);
    // `{k: a}` against `{k: null}`.
    assert_binds_with(
        &map([(key(0), bind(A))], None),
        setup.clone(),
        &Value::map([("k", Value::Null)]),
        [Some(Value::Null), None, None],
    );
    // `{k: null}` against `{}`.
    assert_mismatch_with(
        &map([(key(0), value(PatternComparand::Null))], None),
        setup,
        &Value::from(FrostMap::default()),
    );
}

#[test]
fn a_map_pattern_looks_up_a_key_taken_from_a_local() {
    // `{[input]: a}`
    let pattern = map([(input_key(PatternInput::Local(INPUT)), bind(A))], None);
    let tested = Value::map([(1i64, Value::from("one"))]);
    assert_binds_with(
        &pattern,
        Setup::default().with_input(Value::from(1)),
        &tested,
        [one("one"), None, None],
    );
    assert_mismatch_with(
        &pattern,
        Setup::default().with_input(Value::from(2)),
        &tested,
    );
}

#[test]
fn a_map_pattern_looks_up_a_key_bound_earlier_in_the_pattern() {
    // `{kind: a, [a]: b}`
    let pattern = map(
        [
            (key(0), bind(A)),
            (input_key(PatternInput::EarlierBinding(A)), bind(B)),
        ],
        None,
    );
    let setup = Setup::with_keys(&["kind"]);
    assert_binds_with(
        &pattern,
        setup.clone(),
        &Value::map([("kind", Value::from("x")), ("x", Value::from(5))]),
        [one("x"), one(5), None],
    );
    assert_mismatch_with(
        &pattern,
        setup,
        &Value::map([("kind", Value::from("y")), ("x", Value::from(5))]),
    );
}

#[test]
fn keys_of_every_type_are_looked_up_by_map_key_equality() {
    let keys = [
        MapKey::Bool(true),
        MapKey::Int(1),
        MapKey::Float(float(0.0)),
        MapKey::from("k"),
        MapKey::from(b"k".to_vec()),
    ];
    let tested = Value::from(
        keys.iter()
            .zip(0..)
            .map(|(key, value)| (key.clone(), Value::from(value)))
            .collect::<FrostMap>(),
    );
    let constant_lookup = map([(key(0), bind(A))], None);
    let input_lookup = map([(input_key(PatternInput::Local(INPUT)), bind(A))], None);
    let constant = |key: MapKey| Setup {
        keys: vec![key],
        ..Setup::default()
    };
    let input = |key: Value| Setup::default().with_input(key);

    for (key, value) in keys.iter().zip(0..) {
        let bound = [one(value), None, None];
        assert_binds_with(
            &constant_lookup,
            constant(key.clone()),
            &tested,
            bound.clone(),
        );
        assert_binds_with(
            &input_lookup,
            input(Value::from(key.clone())),
            &tested,
            bound,
        );
    }

    // `0.0` and `-0.0` are distinct keys.
    let negative_zero = float(-0.0);
    assert_mismatch_with(
        &constant_lookup,
        constant(MapKey::Float(negative_zero)),
        &tested,
    );
    assert_mismatch_with(&input_lookup, input(Value::Float(negative_zero)), &tested);
}

#[test]
fn an_invalid_input_key_raises_when_looked_up_in_a_map() {
    for (key, message) in [
        (Value::Null, "Type Null is not a valid Map key"),
        (ints(&[1]), "Type Array is not a valid Map key"),
    ] {
        assert_raises_with(
            &input_key_lookup(),
            Setup::default().with_input(key),
            &Value::map([("k", Value::from(1))]),
            message,
        );
    }
}

#[test]
fn an_invalid_key_bound_earlier_raises_when_looked_up_in_a_map() {
    // `{xs: [...a], [a]: _}`: the rest binds an Array.
    let pattern = map(
        [
            (key(0), at_least([], Some(A))),
            (input_key(PatternInput::EarlierBinding(A)), discard()),
        ],
        None,
    );
    assert_raises_with(
        &pattern,
        Setup::with_keys(&["xs"]),
        &Value::map([("xs", ints(&[1]))]),
        "Type Array is not a valid Map key",
    );
}

#[test]
fn an_invalid_input_key_is_never_looked_up_in_a_value_that_is_not_a_map() {
    assert_mismatch_with(
        &input_key_lookup(),
        Setup::default().with_input(Value::Null),
        &ints(&[1]),
    );
}

#[test]
fn an_invalid_input_key_is_never_looked_up_after_an_earlier_entry_fails() {
    // `{k: _, [input]: _}`, against a Map without `k`.
    let pattern = map(
        [
            (key(0), discard()),
            (input_key(PatternInput::Local(INPUT)), discard()),
        ],
        None,
    );
    assert_mismatch_with(
        &pattern,
        Setup::with_keys(&["k"]).with_input(Value::Null),
        &Value::map([("j", Value::from(1))]),
    );
}

// ---- Alternatives ----

#[test]
fn an_alternative_binds_as_its_first_matching_branch() {
    // `[a, 1] | [1, a]`
    let pattern = either([exact([bind(A), int(1)]), exact([int(1), bind(A)])]);
    assert_binds(&pattern, &ints(&[2, 1]), [one(2), None, None]);
    // The first branch binds `a` to 1 before failing; the second binds it to 3.
    assert_binds(&pattern, &ints(&[1, 3]), [one(3), None, None]);
    // Both branches match; the first wins.
    assert_binds(&pattern, &ints(&[1, 1]), [one(1), None, None]);
    assert_mismatch(&pattern, &ints(&[2, 2]));
}

#[test]
fn an_alternative_commits_to_its_first_matching_branch() {
    // `[[a] | a, 2]`: against `[[1], 3]`, the first branch matches, then `2`
    // fails, and the second branch is not tried.
    assert_mismatch(
        &exact([either([exact([bind(A)]), bind(A)]), int(2)]),
        &Value::array([ints(&[1]), Value::from(3)]),
    );
}

#[test]
fn a_binding_before_an_alternative_outlasts_its_failed_branches() {
    // `[c, [a, 0] | [0, a]]`: the first branch fails after binding `a`, and
    // dropping that binding keeps `c`.
    let pattern = exact([
        bind(C),
        either([exact([bind(A), int(0)]), exact([int(0), bind(A)])]),
    ]);
    assert_binds(
        &pattern,
        &Value::array([Value::from(9), ints(&[0, 5])]),
        [one(5), None, one(9)],
    );
}

#[test]
fn a_later_branch_reads_a_binding_made_before_the_alternative() {
    // `[a, [b, 0] | [b, (a)]]`: the second branch reads `a` after the first
    // branch failed.
    let pattern = exact([
        bind(A),
        either([
            exact([bind(B), int(0)]),
            exact([bind(B), pin(PatternInput::EarlierBinding(A))]),
        ]),
    ]);
    let tested = |last: i64| Value::array([Value::from(7), ints(&[1, last])]);
    assert_binds(&pattern, &tested(7), [one(7), one(1), None]);
    assert_mismatch(&pattern, &tested(8));
}

#[test]
fn bindings_after_an_alternative_follow_the_branch_that_matched() {
    // `[1 | 2, a]`
    let pattern = exact([either([int(1), int(2)]), bind(A)]);
    assert_binds(&pattern, &ints(&[2, 7]), [one(7), None, None]);
    assert_mismatch(&pattern, &ints(&[3, 7]));
}

#[test]
fn an_error_in_a_branch_raises_rather_than_trying_the_next() {
    // `{[input]: _} | _`, with a key that is not a valid Map key.
    assert_raises_with(
        &either([input_key_lookup(), discard()]),
        Setup::default().with_input(Value::Null),
        &Value::map([("k", Value::from(1))]),
        "Type Null is not a valid Map key",
    );
}

#[test]
fn a_branch_after_the_one_that_matches_is_never_tried() {
    // `_ | {[input]: _}`, with a key that is not a valid Map key.
    assert_binds_with(
        &either([discard(), input_key_lookup()]),
        Setup::default().with_input(Value::Null),
        &Value::map([("k", Value::from(1))]),
        [None, None, None],
    );
}

// ---- What a match leaves holding a part, and captures ----

/// The slot of the capture holding the `unshared` probe, in [`run_probed`]'s
/// function.
const UNSHARED: u32 = 0;
/// The slot of the local `a`, in [`run_probed`]'s function.
const PROBED: u32 = 1;

/// Run `code` in a function whose slots are [`UNSHARED`] and [`PROBED`], with
/// `pattern` as its pattern 0, returning its tail value.
fn run_probed(pattern: CompiledPattern, code: Vec<Bytecode>) -> Value {
    let function = func_with_captures(
        [vec![Pop], code].concat(),
        Arity::Exact(0),
        vec![entry("unshared", false), entry("a", false)],
        Vec::new(),
        1,
    );
    let closure = with_patterns(function, vec![pattern])
        .assert_well_formed()
        .close([("unshared", unshared())])
        .expect("`unshared` is supplied");
    Vm::factory()
        .build(closure)
        .run()
        .expect("the program runs")
        .tail()
        .clone()
}

/// `[[1, 2], 3]`, built at runtime, so nothing else holds it or its parts.
fn pair_holding_an_array() -> Vec<Bytecode> {
    vec![
        PushInt(1),
        PushInt(2),
        MakeArray(2),
        PushInt(3),
        MakeArray(2),
    ]
}

#[test]
fn a_bound_part_is_left_held_by_its_local_alone() {
    // `[a, _]`: once the matched value is dropped, `a` holds `[1, 2]` alone.
    let pattern = exact([bind(PROBED), discard()]);
    let code = [
        pair_holding_an_array(),
        vec![
            MatchPattern(0),
            Pop,
            Pop,
            LoadLocal(UNSHARED.into()),
            ConsumeLocal(PROBED.into()),
            Call(1),
        ],
    ]
    .concat();
    assert_eq!(run_probed(pattern, code), Value::from(true));
}

#[test]
fn a_bound_whole_map_is_left_held_by_its_local_alone() {
    // `{} as a`, against a Map built at runtime.
    let pattern = map([], Some(PROBED));
    let code = vec![
        PushInt(1),
        PushInt(2),
        MakeMap(1),
        MatchPattern(0),
        Pop,
        Pop,
        LoadLocal(UNSHARED.into()),
        ConsumeLocal(PROBED.into()),
        Call(1),
    ];
    assert_eq!(run_probed(pattern, code), Value::from(true));
}

#[test]
fn a_bound_whole_array_is_left_held_by_its_local_alone() {
    // `[_, _] as a`, against an Array built at runtime.
    let pattern = array_as(exact([discard(), discard()]), PROBED);
    let code = vec![
        PushInt(1),
        PushInt(2),
        MakeArray(2),
        MatchPattern(0),
        Pop,
        Pop,
        LoadLocal(UNSHARED.into()),
        ConsumeLocal(PROBED.into()),
        Call(1),
    ];
    assert_eq!(run_probed(pattern, code), Value::from(true));
}

#[test]
fn a_mismatch_leaves_nothing_holding_a_part() {
    // `[a, 4]`: `a` is bound to `[1, 2]` before `4` fails. Afterwards, taking the
    // matched value apart hands the probe the only reference to `[1, 2]`.
    let pattern = exact([bind(PROBED), int(4)]);
    let code = [
        vec![LoadLocal(UNSHARED.into())],
        pair_holding_an_array(),
        vec![MatchPattern(0), Pop, ExplodeArray(2), DropBelow(1), Call(1)],
    ]
    .concat();
    assert_eq!(run_probed(pattern, code), Value::from(true));
}

#[test]
fn a_mismatch_leaves_a_local_as_it_was() {
    // `[a, 4]` fails after binding `a`, which keeps its earlier value.
    let pattern = exact([bind(PROBED), int(4)]);
    let code = [
        vec![PushInt(9), DefLocal(PROBED.into())],
        pair_holding_an_array(),
        vec![MatchPattern(0), Pop, Pop, LoadLocal(PROBED.into())],
    ]
    .concat();
    assert_eq!(run_probed(pattern, code), Value::from(9));
}

#[test]
fn a_local_comparand_reads_a_capture() {
    // `(unshared)`, the capture, against the probe itself, then against `null`.
    let pattern = pin(PatternInput::Local(UNSHARED));
    let code = vec![LoadLocal(UNSHARED.into()), MatchPattern(0), DropBelow(1)];
    assert_eq!(run_probed(pattern.clone(), code), Value::from(true));
    let code = vec![PushNull, MatchPattern(0), DropBelow(1)];
    assert_eq!(run_probed(pattern, code), Value::from(false));
}

// ---- Matching in a called frame ----

#[test]
fn a_called_function_matches_against_its_own_patterns_pools_and_slots() {
    // The child, called with 5: `{k: "child", [n]: a} as b`, its pattern 1,
    // against `{k: "child", 5: "five"}`, then against a Map binding `a` to
    // "cinq" instead, then against `null`. Its pools and slots all differ
    // from the caller's, which holds "root" in each of its own slots.
    let child_pattern = map(
        [
            (key(0), value(PatternComparand::Constant(0))),
            (input_key(PatternInput::Local(0)), bind(1)),
        ],
        Some(2),
    );
    let tested = |five: &str| {
        Value::from(
            [
                (MapKey::from("k"), Value::from("child")),
                (MapKey::Int(5), Value::from(five)),
            ]
            .into_iter()
            .collect::<FrostMap>(),
        )
    };
    let child = CompiledFunction {
        constants: vec![
            Value::from("child"),
            tested("five"),
            tested("cinq"),
            Value::Null,
        ],
        key_constants: vec![MapKey::from("k")],
        patterns: vec![discard(), child_pattern],
        ..Arc::unwrap_or_clone(common::func(
            vec![
                DefLocal(0), // n
                Pop,         // the callee
                // The first match binds `a` and `b`.
                LoadConst(1),
                MatchPattern(1),
                MakeArray(2),
                LoadLocal(1),
                LoadLocal(2),
                MakeArray(2),
                // A later match rebinds them; a later mismatch leaves them.
                LoadConst(2),
                MatchPattern(1),
                Pop,
                Pop,
                LoadConst(3),
                MatchPattern(1),
                Pop,
                Pop,
                LoadLocal(1),
                MakeArray(3),
            ],
            Arity::Exact(1),
            vec![entry("n", false), entry("a", false), entry("b", false)],
            Vec::new(),
        ))
    };
    let root = CompiledFunction {
        constants: vec![Value::from("root")],
        key_constants: vec![MapKey::from("other")],
        patterns: vec![bind(0)],
        ..Arc::unwrap_or_clone(common::func(
            vec![
                LoadConst(0),
                DefLocal(0),
                LoadConst(0),
                DefLocal(1),
                LoadConst(0),
                DefLocal(2),
                CreateClosure {
                    child: 0,
                    captures: 0,
                },
                PushInt(5),
                Call(1),
                LoadLocal(0),
                LoadLocal(1),
                LoadLocal(2),
                MakeArray(3),
                MakeArray(2),
            ],
            Arity::Exact(0),
            vec![entry("r0", false), entry("r1", false), entry("r2", false)],
            vec![Arc::new(child)],
        ))
    };
    let result = common::run_fn(Arc::new(root));
    let [child_result, root_slots] = parts(result.tail());
    let [first, bound, later] = parts(&child_result);
    assert_eq!(
        first,
        Value::array([tested("five"), Value::from(true)]),
        "the child's first match"
    );
    assert_eq!(
        bound,
        Value::array([Value::from("five"), tested("five")]),
        "the child's slots, bound by its first match"
    );
    assert_eq!(
        later,
        Value::from("cinq"),
        "a later match rebinds; a mismatch after it leaves the binding"
    );
    assert_eq!(
        root_slots,
        Value::array([
            Value::from("root"),
            Value::from("root"),
            Value::from("root")
        ]),
        "the caller's slots are untouched"
    );
}
