//! Bindings: the values `exec`, `query`, and `fold` give a statement's
//! parameters.

use frostlang::{FrostOpaque, SpecialFloat, Value};
use frostlang_sqlite::{ScriptConnection, connection_factory};

use crate::common::{assert_raises, assert_values, frost, memory_connection, run_app, with_db};

/// The rows of `sql`, queried with `bindings`, as a Frost expression.
fn query(sql: &str, bindings: &str) -> String {
    with_db(&format!("db.query({sql}, {bindings})"))
}

#[test]
fn an_array_binds_its_values_in_order() {
    assert_values(&[
        (
            &query("'SELECT ? AS a, ? AS b'", "[1, 'x']"),
            "[{a: 1, b: 'x'}]",
        ),
        (&query("'SELECT 1 AS a'", "[]"), "[{a: 1}]"),
        // Numbered parameters take the value at their number.
        (
            &query("'SELECT ?2 AS a, ?1 AS b'", "[1, 2]"),
            "[{a: 2, b: 1}]",
        ),
        (&query("'SELECT ?1 AS a, ?1 AS b'", "[7]"), "[{a: 7, b: 7}]"),
        // A number past others counts each before it as a parameter.
        (&query("'SELECT ?3 AS a'", "[1, 2, 3]"), "[{a: 3}]"),
        // Named parameters bind in order too, each name once.
        (
            &query("'SELECT :x AS a, @y AS b'", "[1, 2]"),
            "[{a: 1, b: 2}]",
        ),
        (&query("'SELECT :x AS a, :x AS b'", "[5]"), "[{a: 5, b: 5}]"),
        // Each prefix makes a parameter of its own, which an Array binds
        // apart.
        (
            &query("'SELECT :x AS a, @x AS b'", "[1, 2]"),
            "[{a: 1, b: 2}]",
        ),
    ]);
    assert_raises(&[(
        &query("'SELECT :x AS a, @x AS b'", "[1]"),
        "Function db.query requires 2 bindings, but got 1",
    )]);
}

#[test]
fn a_parameter_number_must_be_in_range() {
    for parameter in ["?0", "?32767"] {
        assert_raises(&[(
            &query(&format!("'SELECT {parameter}'"), "[1]"),
            "Function db.query failed: variable number must be between ?1 and ?32766 (SQLITE_ERROR)",
        )]);
    }
}

#[test]
fn an_array_must_hold_one_value_per_parameter() {
    assert_raises(&[
        (
            &query("'SELECT ?, ?'", "[1]"),
            "Function db.query requires 2 bindings, but got 1",
        ),
        (
            &query("'SELECT ?, ?'", "[1, 2, 3]"),
            "Function db.query requires 2 bindings, but got 3",
        ),
        (
            &query("'SELECT ?'", "[]"),
            "Function db.query requires 1 binding, but got 0",
        ),
        (
            &query("'SELECT 1'", "[1]"),
            "Function db.query requires 0 bindings, but got 1",
        ),
        (
            &query("'SELECT ?3'", "[1]"),
            "Function db.query requires 3 bindings, but got 1",
        ),
    ]);
}

#[test]
fn leaving_out_bindings_binds_none() {
    assert_values(&[(&with_db("db.query('SELECT 1 AS a')"), "[{a: 1}]")]);
    assert_raises(&[
        (
            &with_db("db.query('SELECT ?')"),
            "Function db.query requires 1 binding, but none were given",
        ),
        (
            &with_db("db.query('SELECT :x, :y')"),
            "Function db.query requires 2 bindings, but none were given",
        ),
    ]);
}

#[test]
fn a_map_binds_each_parameter_by_its_name() {
    assert_values(&[
        (
            &query("'SELECT :a AS a, @b AS b, $c AS c'", "{a: 1, b: 2, c: 3}"),
            "[{a: 1, b: 2, c: 3}]",
        ),
        // The name, whatever its prefix.
        (
            &query("'SELECT :id AS a, @id AS b, $id AS c'", "{id: 1}"),
            "[{a: 1, b: 1, c: 1}]",
        ),
        (
            &query("'SELECT :x AS a, :x AS b'", "{x: 5}"),
            "[{a: 5, b: 5}]",
        ),
        (
            &query("'SELECT :x AS a, :y AS b'", "{y: 2, x: 1}"),
            "[{a: 1, b: 2}]",
        ),
        (&query("'SELECT :a_1 AS a'", "{a_1: 1}"), "[{a: 1}]"),
        (&query("'SELECT 1 AS a'", "{}"), "[{a: 1}]"),
        // Names differing only in case are different names.
        (
            &query("'SELECT :a AS x, :A AS y'", "{a: 1, A: 2}"),
            "[{x: 1, y: 2}]",
        ),
        // Names as SQLite allows them.
        (&query("'SELECT $a::b AS x'", "{['a::b']: 1}"), "[{x: 1}]"),
        (
            &query(r"'SELECT :\u{e9} AS x'", r"{['\u{e9}']: 1}"),
            "[{x: 1}]",
        ),
    ]);
}

#[test]
fn a_map_must_match_the_parameters_exactly() {
    assert_raises(&[
        (
            &query("'SELECT :a, :b'", "{a: 1}"),
            "Function db.query requires a binding named 'b', for parameter :b",
        ),
        (
            &query("'SELECT :a'", "{}"),
            "Function db.query requires a binding named 'a', for parameter :a",
        ),
        // A key keeps no prefix.
        (
            &query("'SELECT :a'", "{[':a']: 1}"),
            "Function db.query requires a binding named 'a', for parameter :a",
        ),
        (
            &query("'SELECT :a'", "{a: 1, b: 2}"),
            "Function db.query got a binding named 'b', but the SQL has no parameter of that name",
        ),
        (
            &query("'SELECT 1'", "{b: 2}"),
            "Function db.query got a binding named 'b', but the SQL has no parameter of that name",
        ),
    ]);
    for (key, kind) in [
        ("true", "Bool"),
        ("1", "Int"),
        ("1.5", "Float"),
        ("x'61'", "Bytes"),
    ] {
        assert_raises(&[(
            &query("'SELECT :a'", &format!("{{a: 1, [{key}]: 2}}")),
            &format!("Function db.query requires String keys in a Map of bindings, got {kind}"),
        )]);
    }
}

#[test]
fn a_map_cannot_bind_a_parameter_with_no_name() {
    let nameless = "Function db.query requires an Array of bindings, as the SQL has a parameter not named with :, @, or $";
    assert_raises(&[
        (&query("'SELECT ?'", "{x: 1}"), nameless),
        (&query("'SELECT :a, ?'", "{a: 1}"), nameless),
        (&query("'SELECT ?1'", "{x: 1}"), nameless),
        // The positions a numbered parameter skips are nameless too.
        (&query("'SELECT :a, ?3'", "{a: 1}"), nameless),
    ]);
}

#[test]
fn each_value_binds_as_its_sqlite_type() {
    let cases = [
        ("null", "{type: 'null', value: null}"),
        ("true", "{type: 'integer', value: 1}"),
        ("false", "{type: 'integer', value: 0}"),
        ("42", "{type: 'integer', value: 42}"),
        ("1.5", "{type: 'real', value: 1.5}"),
        ("'text'", "{type: 'text', value: 'text'}"),
        ("x'00ff'", "{type: 'blob', value: x'00ff'}"),
    ];
    for (binding, expected) in cases {
        assert_values(&[
            (
                &query(
                    "'SELECT typeof(?1) AS type, ?1 AS value'",
                    &format!("[{binding}]"),
                ),
                &format!("[{expected}]"),
            ),
            (
                &query(
                    "'SELECT typeof(:v) AS type, :v AS value'",
                    &format!("{{v: {binding}}}"),
                ),
                &format!("[{expected}]"),
            ),
        ]);
    }
}

#[test]
fn every_method_taking_bindings_binds_alike() {
    let source = with_db(
        r"
        db.exec('CREATE TABLE t (a, b)')
        db.exec('INSERT INTO t VALUES (?, ?)', [1, 'x'])
        db.exec('INSERT INTO t VALUES (:a, :b)', {a: 2, b: 'y'})
        [
            db.query('SELECT b FROM t WHERE a = ?', [1]),
            db.query('SELECT b FROM t WHERE a = :a', {a: 2}),
            db.fold('SELECT b FROM t WHERE a = ?', [1], [], fn acc, row -> acc + [row.b]),
            db.fold('SELECT b FROM t WHERE a = :a', {a: 2}, [], fn acc, row -> acc + [row.b]),
        ]
        ",
    );
    assert_values(&[(&source, "[[{b: 'x'}], [{b: 'y'}], ['x'], ['y']]")]);
    for method in ["exec", "query"] {
        assert_raises(&[(
            &with_db(&format!("db.{method}('SELECT ?', [])")),
            &format!("Function db.{method} requires 1 binding, but got 0"),
        )]);
    }
    assert_raises(&[(
        &with_db("db.fold('SELECT ?', [], null, fn acc, row -> acc)"),
        "Function db.fold requires 1 binding, but got 0",
    )]);
}

#[test]
fn only_primitive_values_bind() {
    for (binding, kind) in [("[1]", "Array"), ("{a: 1}", "Map"), ("fn -> 1", "Function")] {
        assert_raises(&[
            (
                &query("'SELECT ?, ?'", &format!("[1, {binding}]")),
                &format!(
                    "Function db.query requires Null, Bool, Int, Float, String, or Bytes for parameter 2, got {kind}"
                ),
            ),
            (
                &query("'SELECT :value'", &format!("{{value: {binding}}}")),
                &format!(
                    "Function db.query requires Null, Bool, Int, Float, String, or Bytes for parameter :value, got {kind}"
                ),
            ),
        ]);
    }
}

/// An Opaque of the tests' own.
#[derive(Debug)]
struct Token;

impl FrostOpaque for Token {
    fn type_name() -> &'static str {
        "Token"
    }

    fn try_to_string(&self) -> Option<String> {
        None
    }
}

/// The result of `source`, run with `app.open_db`, a factory of in-memory
/// connections, and `app.value`, which is `value`.
fn with_value(source: &str, value: Value) -> Result<Value, String> {
    let factory = connection_factory("app.open_db", |_| {
        Ok(ScriptConnection::from(memory_connection()?))
    });
    run_app(source, vec![("open_db", factory), ("value", value)])
        .map_err(|error| error.message().into_owned())
}

#[test]
fn an_infinite_special_float_binds_as_real() {
    for infinity in [f64::INFINITY, f64::NEG_INFINITY] {
        let special = SpecialFloat::new(infinity).expect("an infinity is special");
        let source = r"
            def app = import('app')
            app.open_db().query('SELECT typeof(?1) AS type, ?1 = ?2 AS same', [app.value, app.value])
        ";
        let value = Value::opaque(special);
        assert_eq!(
            with_value(source, value),
            Ok(frost("[{type: 'real', same: 1}]")),
            "{infinity}"
        );
    }
}

#[test]
fn nan_and_other_opaques_do_not_bind() {
    let source = r"
        def app = import('app')
        app.open_db().query('SELECT ?', [app.value])
    ";
    let nan = SpecialFloat::new(f64::NAN).expect("NaN is special");
    assert_eq!(
        with_value(source, Value::opaque(nan)),
        Err("Function db.query cannot bind NaN to parameter 1".to_owned())
    );
    assert_eq!(
        with_value(
            r"
            def app = import('app')
            app.open_db().query('SELECT :x', {x: app.value})
            ",
            Value::opaque(nan)
        ),
        Err("Function db.query cannot bind NaN to parameter :x".to_owned())
    );
    assert_eq!(
        with_value(source, Value::opaque(Token)),
        Err(
            "Function db.query requires Null, Bool, Int, Float, String, or Bytes for parameter 1, got Opaque"
                .to_owned()
        )
    );
}

#[test]
fn a_failed_binding_runs_nothing() {
    let source = with_db(
        r"
        db.exec('CREATE TABLE t (a, b)')
        try_call(db.exec, ['INSERT INTO t VALUES (?, ?)', [1, [2]]])
        try_call(db.exec, ['INSERT INTO t VALUES (:a, :b)', {a: 1}])
        try_call(db.fold, ['INSERT INTO t VALUES (?, ?) RETURNING a', [1], null, fn acc, row -> acc])
        db.query('SELECT count(*) AS c FROM t')
        ",
    );
    assert_values(&[(&source, "[{c: 0}]")]);
}

#[test]
fn bindings_are_an_array_or_a_map() {
    for (argument, kind) in [
        ("null", "Null"),
        ("true", "Bool"),
        ("1", "Int"),
        ("1.5", "Float"),
        ("'text'", "String"),
        ("x'00'", "Bytes"),
        ("fn -> 1", "Function"),
    ] {
        for method in ["exec", "query"] {
            assert_raises(&[(
                &with_db(&format!("db.{method}('SELECT 1', {argument})")),
                &format!(
                    "Function db.{method} requires Structured as argument 2 (bindings), got {kind}"
                ),
            )]);
        }
        assert_raises(&[(
            &with_db(&format!(
                "db.fold('SELECT 1', {argument}, null, fn acc, row -> acc)"
            )),
            &format!("Function db.fold requires Structured as argument 2 (bindings), got {kind}"),
        )]);
    }
    assert_eq!(
        with_value(
            r"
            def app = import('app')
            app.open_db().query('SELECT 1', app.value)
            ",
            Value::opaque(Token)
        ),
        Err(
            "Function db.query requires Structured as argument 2 (bindings), got Opaque".to_owned()
        )
    );
}
