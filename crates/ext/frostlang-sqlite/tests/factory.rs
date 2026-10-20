//! Host connection factories: a host giving scripts its own connections,
//! without `ext.sqlite`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use frostlang::{FrostError, MapKey, Value, frost_closure};
use frostlang_sqlite::rusqlite::{self, Connection, ffi};
use frostlang_sqlite::{EnumSet, Method, ScriptConnection, connection_factory, sqlite_error};

use crate::common::{Pooled, app_raises, app_value, frost, memory_connection, run_app};

/// A factory of in-memory connections exposing `methods`.
fn memory_factory(methods: EnumSet<Method>) -> Value {
    connection_factory("app.open_db", move |_| {
        Ok(ScriptConnection::from(memory_connection()?).with_methods(methods))
    })
}

#[test]
fn a_factory_serves_scripts_without_the_extension() {
    let source = r"
        def db = import('app').open_db()
        db.script('CREATE TABLE t (x); INSERT INTO t VALUES (1), (2)')
    ";
    assert_eq!(
        app_value(source, memory_factory(EnumSet::all())),
        Value::Int(2)
    );
    assert_eq!(
        app_raises("import('ext.sqlite')", memory_factory(EnumSet::all())),
        "Could not resolve import 'ext.sqlite'",
        "the extension is not installed"
    );
}

#[test]
fn each_call_makes_a_fresh_connection() {
    let made = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&made);
    let factory = connection_factory("app.open_db", move |_| {
        counted.fetch_add(1, Ordering::SeqCst);
        Ok(memory_connection()?.into())
    });
    let source = r"
        def app = import('app')
        app.open_db().script('CREATE TABLE t (x)')
        try_call(app.open_db().script, ['INSERT INTO t VALUES (1)']).error
    ";
    assert_eq!(
        app_value(source, factory),
        Value::from("Function db.script failed: no such table: t (SQLITE_ERROR)"),
        "the second connection's database is its own"
    );
    assert_eq!(made.load(Ordering::SeqCst), 2, "each call ran the factory");
}

#[test]
fn a_factory_receives_the_argument_it_is_called_with() {
    let received = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&received);
    let factory = connection_factory("app.open_db", move |argument| {
        recorded.lock().expect("no test panicked").push(argument);
        Ok(memory_connection()?.into())
    });
    let source = r"
        def app = import('app')
        app.open_db()
        app.open_db('reports')
        app.open_db(null)
        app.open_db([1, 2])
    ";
    app_value(source, factory);
    assert_eq!(
        *received.lock().expect("no test panicked"),
        [
            None,
            Some(Value::from("reports")),
            Some(Value::Null),
            Some(frost("[1, 2]"))
        ],
        "each call's argument, if any"
    );
}

#[test]
fn a_factory_can_choose_a_database_by_its_argument() {
    let factory = connection_factory("app.open_db", |argument| match argument {
        Some(Value::String(name)) if name.as_str() == "scratch" => Ok(memory_connection()?.into()),
        Some(other) => Err(FrostError::from_string(format!(
            "there is no database {}",
            other.to_frost_string()
        ))),
        None => Err(FrostError::from_static("name a database")),
    });
    let source = r"
        def app = import('app')
        [
            app.open_db('scratch').script('CREATE TABLE t (x); INSERT INTO t VALUES (1)'),
            try_call(app.open_db, ['reports']).error,
            try_call(app.open_db).error,
        ]
    ";
    assert_eq!(
        app_value(source, factory),
        frost(r"[1, 'there is no database reports', 'name a database']")
    );
}

#[test]
fn a_bundle_offers_every_method_by_default() {
    assert_eq!(
        app_value(
            "sorted(keys(import('app').open_db()))",
            memory_factory(EnumSet::all())
        ),
        frost("['close', 'script']")
    );
}

#[test]
fn a_bundle_offers_only_the_methods_chosen() {
    let cases = [
        (EnumSet::only(Method::Script), "['script']"),
        (EnumSet::only(Method::Close), "['close']"),
        (EnumSet::all() - Method::Script, "['close']"),
        (EnumSet::all() - Method::Close, "['script']"),
        (Method::Script | Method::Close, "['close', 'script']"),
        (EnumSet::empty(), "[]"),
    ];
    for (methods, expected) in cases {
        assert_eq!(
            app_value(
                "sorted(keys(import('app').open_db()))",
                memory_factory(methods)
            ),
            frost(expected),
            "{methods:?}"
        );
    }
}

#[test]
fn a_trimmed_bundle_still_works() {
    let source = r"
        import('app').open_db().script('CREATE TABLE t (x); INSERT INTO t VALUES (1)')
    ";
    assert_eq!(
        app_value(source, memory_factory(EnumSet::only(Method::Script))),
        Value::Int(1)
    );
}

#[test]
fn a_factory_error_is_the_calls_error() {
    let denied = connection_factory("app.open_db", |_| {
        Err(FrostError::from_static("no database today"))
    });
    assert_eq!(
        app_raises("import('app').open_db()", denied),
        "no database today"
    );

    // A thrown value arrives as that value.
    let thrown = connection_factory("app.open_db", |_| {
        Err(FrostError::from_value(Value::from_iter([(
            MapKey::from("reason"),
            Value::from("denied"),
        )])))
    });
    assert_eq!(
        app_value("try_call(import('app').open_db).error", thrown),
        frost("{reason: 'denied'}")
    );
}

#[test]
fn the_extension_configures_nothing_on_a_hosts_connections() {
    // `ext.sqlite` turns foreign keys on for its own connections, but leaves a
    // host's as the host made them.
    let factory = connection_factory("app.open_db", |_| {
        let connection = memory_connection()?;
        connection
            .pragma_update(None, "foreign_keys", false)
            .map_err(|err| sqlite_error("app.open_db", &err))?;
        Ok(connection.into())
    });
    let source = r"
        def db = import('app').open_db()
        db.script('CREATE TABLE parent (id INTEGER PRIMARY KEY); CREATE TABLE child (parent INTEGER REFERENCES parent (id))')
        db.script('INSERT INTO child VALUES (1)')
    ";
    assert_eq!(app_value(source, factory), Value::Int(1));
}

#[test]
fn a_factory_takes_at_most_one_argument() {
    assert_eq!(
        app_raises(
            "import('app').open_db(1, 2)",
            memory_factory(EnumSet::all())
        ),
        "Function app.open_db expects between 0 and 1 arguments, but was called with 2"
    );
}

/// For each pooled connection released so far, in order, whether it had no
/// transaction open.
type Releases = Arc<Mutex<Vec<bool>>>;

/// `app.open_db`, a factory of pooled connections exposing `methods`, and
/// `app.released`, how many of them have been released; with the record of
/// their releases.
fn pooled_app(methods: EnumSet<Method>) -> (Vec<(&'static str, Value)>, Releases) {
    let releases = Releases::default();
    let recorded = Arc::clone(&releases);
    let factory = connection_factory("app.open_db", move |_| {
        let recorded = Arc::clone(&recorded);
        let pooled = Pooled::new(memory_connection()?, move |connection| {
            recorded
                .lock()
                .expect("no test panicked")
                .push(connection.is_autocommit());
        });
        Ok(ScriptConnection::pooled(pooled).with_methods(methods))
    });
    let counted = Arc::clone(&releases);
    let released = frost_closure!("app.released", move || {
        i64::try_from(counted.lock().expect("no test panicked").len())
            .expect("few connections are made")
    });
    (vec![("open_db", factory), ("released", released)], releases)
}

/// The value of `source`, run with [`pooled_app`], and the record of its
/// connections' releases.
fn run_pooled(source: &str, methods: EnumSet<Method>) -> (Value, Vec<bool>) {
    let (entries, releases) = pooled_app(methods);
    let result = run_app(source, entries)
        .unwrap_or_else(|error| panic!("{source:?} should run, but raised: {error}"));
    let releases = releases.lock().expect("no test panicked").clone();
    (result, releases)
}

#[test]
fn a_pooled_connection_works_as_an_owned_one() {
    let source = r"
        def db = import('app').open_db()
        [
            db.script('CREATE TABLE t (x); INSERT INTO t VALUES (1)'),
            try_call(db.script, ['INSERT INTO missing VALUES (1)']).error,
        ]
    ";
    let (result, _) = run_pooled(source, EnumSet::all());
    assert_eq!(
        result,
        frost("[1, 'Function db.script failed: no such table: missing (SQLITE_ERROR)']")
    );
}

#[test]
fn closing_a_pooled_connection_releases_it() {
    let source = r"
        def app = import('app')
        def db = app.open_db()
        def before = app.released()
        db.close()
        [before, app.released(), try_call(db.script, ['SELECT 1']).error]
    ";
    let (result, releases) = run_pooled(source, EnumSet::all());
    assert_eq!(
        result,
        frost("[0, 1, 'Function db.script requires an open connection, but it is closed']")
    );
    assert_eq!(releases, [true], "released once, with no transaction open");
}

#[test]
fn a_pooled_connection_is_released_once_no_script_holds_its_bundle() {
    for methods in [EnumSet::all(), EnumSet::all() - Method::Close] {
        let source = r"
            def app = import('app')
            def db = app.open_db()
            [app.released(), db.script('SELECT 1')]
        ";
        let (result, releases) = run_pooled(source, methods);
        assert_eq!(
            result,
            frost("[0, 0]"),
            "{methods:?}: the connection is held while its bundle is"
        );
        assert_eq!(
            releases,
            [true],
            "{methods:?}: released by the time the script's Vm is gone"
        );
    }
}

#[test]
fn a_pooled_connection_returns_to_its_pool_with_no_transaction_open() {
    for release in ["db.close()", "null"] {
        let source = format!(
            r"
            def db = import('app').open_db()
            db.script('CREATE TABLE t (x); BEGIN; INSERT INTO t VALUES (1)')
            {release}
            "
        );
        let (_, releases) = run_pooled(&source, EnumSet::all());
        assert_eq!(
            releases,
            [true],
            "after {release:?}, the open transaction was rolled back"
        );
    }
}

#[test]
fn a_script_connection_shows_how_it_is_exposed() {
    let connection = || Connection::open_in_memory().expect("an in-memory database opens");
    let cases = [
        (
            ScriptConnection::new(connection()).with_methods(EnumSet::only(Method::Script)),
            "ScriptConnection { methods: EnumSet(Script), authorizer: None, filters_sql: false, .. }",
        ),
        (
            ScriptConnection::new(connection())
                .with_methods(EnumSet::empty())
                .with_authorizer(frostlang_sqlite::Authorizer::new().deny_attach())
                .with_sql_filter(|_| Ok(())),
            "ScriptConnection { methods: EnumSet(), authorizer: Some(Authorizer { deny_writes: \
             false, deny_attach: true, pragmas: None, allowed_functions: None, denied_functions: \
             {}, rules: 0 }), filters_sql: true, .. }",
        ),
    ];
    for (connection, expected) in cases {
        assert_eq!(format!("{connection:?}"), expected);
    }
}

#[test]
fn sqlite_error_words_errors_as_the_extension_does() {
    let failure = |code, message: Option<&str>| {
        rusqlite::Error::SqliteFailure(ffi::Error::new(code), message.map(str::to_string))
    };
    let cases = [
        (
            Connection::open_with_flags(
                "/no/such/directory/app.db",
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .expect_err("there is no such database"),
            "Function app.open_db failed: unable to open database file: \
             /no/such/directory/app.db (SQLITE_CANTOPEN)",
        ),
        // An extended code is named by its primary code.
        (
            failure(
                ffi::SQLITE_CONSTRAINT_UNIQUE,
                Some("UNIQUE constraint failed: t.id"),
            ),
            "Function app.open_db failed: UNIQUE constraint failed: t.id (SQLITE_CONSTRAINT)",
        ),
        // Without a message, SQLite's description of the code.
        (
            failure(ffi::SQLITE_BUSY, None),
            "Function app.open_db failed: database is locked (SQLITE_BUSY)",
        ),
        // A code SQLite has no name for.
        (
            failure(99, Some("odd")),
            "Function app.open_db failed: odd (SQLite result code 99)",
        ),
        // An error of rusqlite's own.
        (
            rusqlite::Error::InvalidQuery,
            "Function app.open_db failed: Query is not read-only",
        ),
    ];
    for (err, expected) in cases {
        assert_eq!(
            sqlite_error("app.open_db", &err).message(),
            expected,
            "{err:?}"
        );
    }
}
