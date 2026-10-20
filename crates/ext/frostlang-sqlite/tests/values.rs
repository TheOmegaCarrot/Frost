//! Reading SQLite values as Frost values, and values making the round trip
//! through a database and back.

use frostlang::{SpecialFloat, Value};

use crate::common::{Scratch, assert_raises, assert_values, value, with_db};

#[test]
fn each_sqlite_type_reads_as_its_frost_type() {
    assert_values(&[(
        &with_db(r#"db.query("SELECT NULL AS n, 42 AS i, 1.5 AS f, 'text' AS s, x'00ff' AS b")"#),
        "[{n: null, i: 42, f: 1.5, s: 'text', b: x'00ff'}]",
    )]);
    // A whole REAL stays a Float.
    let source = with_db("db.query('SELECT 2.0 AS f')[0].f");
    assert_eq!(value(&source), value("2.0"));
    assert_ne!(value(&source), value("2"), "2.0 is not the Int 2");
}

/// `values`, a Frost Array, inserted into a column of `column_type` and read
/// back.
fn round_trip(column_type: &str, values: &str) -> String {
    with_db(&format!(
        r"
        db.exec('CREATE TABLE t (id INTEGER PRIMARY KEY, v {column_type})')
        def values = {values}
        map values with fn v -> db.exec('INSERT INTO t (v) VALUES (?)', [v])
        map db.query('SELECT v FROM t ORDER BY id') with fn row -> row.v
        "
    ))
}

#[test]
fn values_make_the_round_trip() {
    let values = r"[
        null,
        0,
        -1,
        9223372036854775807,
        -9223372036854775807 - 1,
        0.0,
        -0.0,
        0.1,
        -1.5e300,
        5e-324,
        1.7976931348623157e308,
        -1.7976931348623157e308,
        '',
        'text',
        'a\0b',
        'snow: \u{2744}',
        x'',
        x'00',
        x'00ff7f80',
    ]";
    // A column without a type keeps each value as it is bound.
    assert_values(&[(&round_trip("", values), values)]);
}

#[test]
fn large_values_make_the_round_trip_whole() {
    // A MiB of each, made by SQLite, read, bound back, and compared there.
    let source = with_db(
        r#"
        def make = "SELECT zeroblob(1048576) AS b, printf('%.*c', 1048576, 'x') AS s"
        def [made] = db.query(make)
        def [back] = db.query("SELECT ?1 = zeroblob(1048576) AS b, ?2 = printf('%.*c', 1048576, 'x') AS s", [made.b, made.s])
        [len(made.b), len(made.s), back]
        "#,
    );
    assert_values(&[(&source, "[1048576, 1048576, {b: 1, s: 1}]")]);
}

#[test]
fn bools_read_back_as_ints() {
    assert_values(&[(&round_trip("", "[true, false]"), "[1, 0]")]);
}

#[test]
fn a_column_type_converts_as_sqlite_converts() {
    // SQLite's type affinity, which a column's declared type gives it.
    assert_values(&[
        (&round_trip("INTEGER", "[1.0, '2', 2.5]"), "[1, 2, 2.5]"),
        (&round_trip("REAL", "[1, '2.5']"), "[1.0, 2.5]"),
        (&round_trip("TEXT", "[1, 1.5]"), "['1', '1.5']"),
        (&round_trip("BLOB", "[1, 'text']"), "[1, 'text']"),
        // SQLite keeps a whole REAL as an integer, so -0.0 loses its sign.
        (&round_trip("REAL", "[-0.0]"), "[0.0]"),
    ]);
}

#[test]
fn text_and_bytes_stay_apart() {
    let source = with_db(
        r#"
        db.query("SELECT typeof(?1) AS a, typeof(?2) AS b, ?1 = ?2 AS same", ['ab', x'6162'])
        "#,
    );
    assert_values(&[(&source, "[{a: 'text', b: 'blob', same: 0}]")]);
}

#[test]
fn an_infinite_real_reads_as_a_special_float() {
    let row = value(&with_db("db.query('SELECT 9e999 AS a, -9e999 AS b')[0]"));
    for (key, expected) in [("a", f64::INFINITY), ("b", f64::NEG_INFINITY)] {
        let Value::Map(ref row) = row else {
            panic!("a row is a Map, not {row:?}");
        };
        let column = row.get(&key.into()).expect("the row has the column");
        assert_eq!(
            column
                .downcast_opaque::<SpecialFloat>()
                .map(|special| special.get()),
            Some(expected),
            "{key}: {column:?}"
        );
    }
}

#[test]
fn an_infinity_makes_the_round_trip() {
    // Read as a SpecialFloat, bound back, it is the same REAL.
    let source = with_db(
        r"
        db.exec('CREATE TABLE t (v)')
        def [row] = db.query('SELECT 9e999 AS a, -9e999 AS b')
        db.exec('INSERT INTO t VALUES (?), (?)', [row.a, row.b])
        db.query('SELECT typeof(v) AS type, v = 9e999 AS positive, v = -9e999 AS negative FROM t ORDER BY rowid')
        ",
    );
    assert_values(&[(
        &source,
        "[{type: 'real', positive: 1, negative: 0}, {type: 'real', positive: 0, negative: 1}]",
    )]);
}

#[test]
fn sqlite_makes_nan_null() {
    assert_values(&[(
        &with_db("db.query('SELECT 9e999 - 9e999 AS n')"),
        "[{n: null}]",
    )]);
}

#[test]
fn text_that_is_not_utf8_is_an_error() {
    let message = "requires TEXT to be UTF-8, but column 't' holds TEXT that is not";
    assert_raises(&[
        (
            &with_db(r#"db.query("SELECT CAST(x'ff' AS TEXT) AS t")"#),
            &format!("Function db.query {message}"),
        ),
        (
            &with_db(r#"db.fold("SELECT CAST(x'61ff' AS TEXT) AS t", null, fn acc, row -> acc)"#),
            &format!("Function db.fold {message}"),
        ),
    ]);
    // Read as a BLOB, the same bytes are fine.
    assert_values(&[(
        &with_db(r#"db.query("SELECT CAST(CAST(x'ff' AS TEXT) AS BLOB) AS t")"#),
        "[{t: x'ff'}]",
    )]);
}

#[test]
fn a_column_name_that_is_not_utf8_is_an_error() {
    // A schema SQLite never checked, as a file another program wrote may
    // hold: the table's one column is named by the byte 0xff.
    let scratch = Scratch::new();
    let path = scratch.literal("odd.db");
    let source = format!(
        r#"
        def setup = sqlite.open({path})
        setup.script("CREATE TABLE t (a); INSERT INTO t VALUES (1); PRAGMA writable_schema = ON; UPDATE sqlite_schema SET sql = 'CREATE TABLE t (' || CAST(x'ff' AS TEXT) || ')' WHERE name = 't'; PRAGMA writable_schema = OFF")
        setup.close()
        def db = sqlite.open({path})
        [
            try_call(db.query, ['SELECT * FROM t']).error,
            try_call(db.fold, ['SELECT * FROM t', null, fn acc, row -> acc]).error,
            db.query('SELECT count(*) AS c FROM t'),
        ]
        "#
    );
    let message = "requires column names to be UTF-8, but column 1 has a name that is not";
    assert_values(&[(
        &source,
        &format!("['Function db.query {message}', 'Function db.fold {message}', [{{c: 1}}]]"),
    )]);
}

#[test]
fn a_fold_stops_at_the_first_row_it_cannot_read() {
    let source = with_db(
        r#"
        db.exec('CREATE TABLE t (id INTEGER PRIMARY KEY, v)')
        db.exec("INSERT INTO t (v) VALUES ('a'), (CAST(x'ff' AS TEXT)), ('c')")
        db.exec('CREATE TABLE log (id)')
        def folded = try_call(db.fold, ['SELECT id, v FROM t ORDER BY id', null, fn acc, row -> db.exec('INSERT INTO log VALUES (?)', [row.id])])
        [folded.ok, db.query('SELECT id FROM log')]
        "#,
    );
    // The first row reached the callback, and no row after the second.
    assert_values(&[(&source, "[false, [{id: 1}]]")]);
}
