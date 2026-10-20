//! Authorizers: the SQL a host lets a connection's scripts run.

use std::sync::{Arc, Mutex};

use frostlang::Value;
use frostlang_sqlite::rusqlite::Connection;
use frostlang_sqlite::rusqlite::hooks::{
    AuthAction, AuthContext, Authorization, TransactionOperation,
};
use frostlang_sqlite::rusqlite::types::ValueRef;
use frostlang_sqlite::{
    Authorizer, REPORTING_PRAGMAS, ScriptConnection, connection_factory, sqlite_error,
};

use crate::common::{
    Pooled, Scratch, app_raises, app_value, frost, memory_connection, run_app, string_literal,
};

/// How a script's denied statement fails.
const DENIED: &str = "Function db.script failed: not authorized (SQLITE_AUTH)";

/// A database file of one test's own: a table `t`, with an index and a
/// trigger logging its inserts to `log`, a view `v` over it, an empty table
/// `copy`, and a table `p` with a column named `temp`.
struct Fixture {
    scratch: Scratch,
}

impl Fixture {
    fn new() -> Self {
        let fixture = Self {
            scratch: Scratch::new(),
        };
        fixture.set_up(
            r#"
            CREATE TABLE t (x, secret);
            CREATE INDEX t_x ON t (x);
            INSERT INTO t VALUES (1, 'hush');
            CREATE TABLE log (x);
            CREATE TRIGGER logged AFTER INSERT ON t BEGIN INSERT INTO log VALUES (new.x); END;
            CREATE VIEW v AS SELECT x, secret FROM t;
            CREATE TABLE copy (a, b);
            CREATE TABLE p (a, "temp");
            "#,
        );
        fixture
    }

    /// Runs `sql` on the database, outside any authorizer.
    fn set_up(&self, sql: &str) {
        Connection::open(self.scratch.path("app.db"))
            .and_then(|connection| connection.execute_batch(sql))
            .expect("the fixture is set up");
    }

    /// The path of the database file.
    fn path(&self) -> std::path::PathBuf {
        self.scratch.path("app.db")
    }

    /// A factory of connections to the database, each under `authorizer`.
    fn factory(&self, authorizer: &Authorizer) -> Value {
        let path = self.path();
        let authorizer = authorizer.clone();
        connection_factory("app.open_db", move |_| {
            let connection =
                Connection::open(&path).map_err(|err| sqlite_error("app.open_db", &err))?;
            Ok(ScriptConnection::new(connection).with_authorizer(authorizer.clone()))
        })
    }

    /// What `sql` reads from the database, outside any authorizer: each row
    /// as its columns rendered as SQL literals.
    fn read(&self, sql: &str) -> Vec<Vec<String>> {
        let connection = Connection::open(self.path()).expect("the database opens");
        let mut statement = connection.prepare(sql).expect("the query prepares");
        let columns = statement.column_count();
        statement
            .query_map([], |row| {
                (0..columns)
                    .map(|column| row.get_ref(column).map(literal))
                    .collect()
            })
            .expect("the query runs")
            .collect::<Result<_, _>>()
            .expect("each row reads")
    }

    /// Everything a write could change: the schema, every row, and the
    /// database's settings.
    fn snapshot(&self) -> Vec<Vec<Vec<String>>> {
        [
            "SELECT type, name, sql FROM sqlite_schema ORDER BY name",
            "SELECT * FROM t",
            "SELECT * FROM log",
            "SELECT * FROM copy",
            "SELECT * FROM p",
            "PRAGMA user_version",
            "PRAGMA journal_mode",
        ]
        .map(|sql| self.read(sql))
        .into()
    }
}

/// `value` as a SQL literal.
fn literal(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => "NULL".to_string(),
        ValueRef::Integer(int) => int.to_string(),
        ValueRef::Real(float) => float.to_string(),
        ValueRef::Text(text) => format!("'{}'", String::from_utf8_lossy(text)),
        ValueRef::Blob(blob) => format!(
            "x'{}'",
            blob.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
    }
}

/// `sql` as a script runs it on a connection from `app.open_db`.
fn script(sql: &str) -> String {
    format!("import('app').open_db().script({})", string_literal(sql))
}

/// Assert `authorizer` denies each statement of `sqls`, each on a database of
/// its own, which the denial leaves as it was.
fn assert_denies(authorizer: &Authorizer, sqls: &[&str]) {
    for sql in sqls {
        let fixture = Fixture::new();
        let before = fixture.snapshot();
        assert_eq!(
            app_raises(&script(sql), fixture.factory(authorizer)),
            DENIED,
            "{authorizer:?} denies {sql:?}"
        );
        assert_eq!(
            fixture.snapshot(),
            before,
            "denying {sql:?} changed nothing"
        );
    }
}

/// Assert `authorizer` allows each statement of `sqls`, each on a database of
/// its own.
fn assert_allows(authorizer: &Authorizer, sqls: &[&str]) {
    for sql in sqls {
        let fixture = Fixture::new();
        app_value(&script(sql), fixture.factory(authorizer));
    }
}

/// Assert `authorizer` allows `sql`, and that it then changed the rows of `t`
/// to `rows`.
fn assert_writes(authorizer: &Authorizer, sql: &str, rows: &[[&str; 2]]) {
    let fixture = Fixture::new();
    app_value(&script(sql), fixture.factory(authorizer));
    assert_eq!(
        fixture.read("SELECT * FROM t ORDER BY x"),
        rows,
        "{authorizer:?} let {sql:?} take effect"
    );
}

/// Statements no helper denies but `deny_writes`.
const READS: &[&str] = &[
    "SELECT * FROM t",
    "SELECT * FROM v",
    "SELECT name FROM sqlite_schema",
    "WITH RECURSIVE c(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM c WHERE n < 3) SELECT n FROM c",
    "BEGIN; SAVEPOINT s; RELEASE s; COMMIT",
    "BEGIN; ROLLBACK",
];

/// Attaching a file named by an expression rather than a string literal,
/// which SQLite judges with no name, each with `{path}` for the file.
const COMPUTED_ATTACHES: &[&str] = &[
    "ATTACH {path} || '' AS o",
    "ATTACH CAST({path} AS TEXT) AS o",
    "ATTACH printf('%s', {path}) AS o",
    "ATTACH (SELECT {path}) AS o",
    "ATTACH CASE WHEN 1 THEN {path} END AS o",
];

#[test]
fn a_new_authorizer_allows_everything() {
    let authorizer = Authorizer::new();
    assert_allows(&authorizer, READS);
    assert_allows(
        &authorizer,
        &[
            "PRAGMA user_version = 5",
            "ATTACH '' AS other",
            "VACUUM",
            "SELECT upper('a')",
        ],
    );
    assert_writes(
        &authorizer,
        "INSERT INTO t VALUES (2, 'x')",
        &[["1", "'hush'"], ["2", "'x'"]],
    );
}

#[test]
fn deny_writes_denies_changing_any_database_but_temp() {
    let authorizer = Authorizer::new().deny_writes();
    assert_denies(
        &authorizer,
        &[
            // Data, in any case.
            "INSERT INTO t VALUES (2, 'x')",
            "insert into T values (2, 'x')",
            "UPDATE t SET x = 2",
            "DELETE FROM t",
            "INSERT INTO copy SELECT * FROM t",
            // Schema.
            "CREATE TABLE u (x)",
            "DROP TABLE log",
            "ALTER TABLE t ADD COLUMN y",
            "ALTER TABLE t RENAME TO u",
            r#"ALTER TABLE p DROP COLUMN "temp""#,
            "CREATE INDEX i ON t (x)",
            "CREATE VIEW w AS SELECT 1",
            "DROP VIEW v",
            "CREATE TRIGGER again AFTER DELETE ON t BEGIN DELETE FROM log; END",
            "DROP TRIGGER logged",
            // Rewriting and maintenance.
            "VACUUM",
            "REINDEX",
            "ANALYZE",
            // Pragmas setting a value, of any database or the connection, or
            // writing without one.
            "PRAGMA user_version = 5",
            "PRAGMA USER_VERSION = 5",
            "PRAGMA temp.user_version = 5",
            "PRAGMA journal_mode = WAL",
            "PRAGMA query_only = 0",
            "PRAGMA foreign_keys = OFF",
            "PRAGMA temp.cache_size = 10",
            "PRAGMA optimize",
            "PRAGMA incremental_vacuum",
            "PRAGMA wal_checkpoint",
            // An attached database is no exception.
            "ATTACH '' AS other; CREATE TABLE other.u (x)",
        ],
    );
}

#[test]
fn deny_writes_allows_reading_transactions_and_temporary_tables() {
    let authorizer = Authorizer::new().deny_writes();
    assert_allows(&authorizer, READS);
    assert_allows(
        &authorizer,
        &[
            "CREATE TEMP TABLE scratch (x); INSERT INTO scratch SELECT x FROM t; UPDATE scratch SET x = 2; DELETE FROM scratch; DROP TABLE scratch",
            "CREATE TEMP VIEW tv AS SELECT x FROM t; DROP VIEW tv",
            "CREATE TEMP TABLE scratch (x); CREATE INDEX temp.i ON scratch (x)",
            "CREATE TEMP TABLE scratch (x, y); ALTER TABLE scratch ADD COLUMN z; ALTER TABLE scratch RENAME COLUMN x TO w; ALTER TABLE scratch DROP COLUMN y; ALTER TABLE scratch RENAME TO renamed",
            // Reading a setting, and reporting on a table.
            "PRAGMA user_version",
            "PRAGMA table_info(t)",
            "PRAGMA integrity_check",
            "ATTACH '' AS other",
        ],
    );
}

#[test]
fn deny_writes_denies_writes_made_through_a_trigger() {
    // A temporary trigger is allowed, but the write it would make is not.
    let authorizer = Authorizer::new().deny_writes();
    let fixture = Fixture::new();
    let source = r"
        def db = import('app').open_db()
        db.script('CREATE TEMP TABLE scratch (x); CREATE TEMP TRIGGER leak AFTER INSERT ON scratch BEGIN INSERT INTO main.copy VALUES (new.x, 1); END')
        db.script('INSERT INTO scratch VALUES (1)')
    ";
    assert_eq!(app_raises(source, fixture.factory(&authorizer)), DENIED);
    assert!(
        fixture.read("SELECT * FROM copy").is_empty(),
        "the trigger wrote nothing"
    );
}

/// As `deny_writes` documents, attaching is `deny_attach`'s to deny.
#[test]
fn deny_writes_alone_leaves_attaching_files_allowed() {
    let authorizer = Authorizer::new().deny_writes();
    for sql in ["ATTACH {path} AS o"].iter().chain(COMPUTED_ATTACHES) {
        let fixture = Fixture::new();
        let sql = sql.replace("{path}", &fixture.scratch.literal("other.db"));
        app_value(&script(&sql), fixture.factory(&authorizer));
        assert!(
            fixture.scratch.path("other.db").exists(),
            "{sql:?} attached the file"
        );
    }
    // A copy is denied, but leaves an empty file where it would have gone.
    let fixture = Fixture::new();
    let sql = format!("VACUUM INTO {}", fixture.scratch.literal("copy.db"));
    assert_eq!(
        app_raises(&script(&sql), fixture.factory(&authorizer)),
        DENIED
    );
    let copy = fixture.scratch.path("copy.db");
    assert_eq!(
        std::fs::metadata(&copy).map(|file| file.len()).ok(),
        Some(0),
        "an empty file is left"
    );
}

#[test]
fn deny_attach_denies_reaching_other_files() {
    let authorizer = Authorizer::new().deny_attach();
    let forms = [
        "ATTACH {path} AS o",
        "ATTACH 'file:' || {path} || '?mode=rwc' AS o",
        "VACUUM INTO {path}",
        "VACUUM main INTO {path}",
    ];
    for sql in forms.iter().chain(COMPUTED_ATTACHES) {
        let fixture = Fixture::new();
        let sql = sql.replace("{path}", &fixture.scratch.literal("other.db"));
        assert_eq!(
            app_raises(&script(&sql), fixture.factory(&authorizer)),
            DENIED,
            "{sql:?}"
        );
        assert!(
            !fixture.scratch.path("other.db").exists(),
            "denying {sql:?} made no file"
        );
    }
    assert_denies(
        &authorizer,
        &[
            "ATTACH ':memory:' AS o",
            "ATTACH NULL AS o",
            "ATTACH '' AS o; DETACH o",
        ],
    );
}

#[test]
fn deny_attach_allows_a_private_temporary_database() {
    let authorizer = Authorizer::new().deny_attach();
    assert_allows(&authorizer, READS);
    assert_allows(
        &authorizer,
        &["ATTACH '' AS other; CREATE TABLE other.u (x)", "VACUUM"],
    );
    assert_writes(
        &authorizer,
        "INSERT INTO t VALUES (2, 'x')",
        &[["1", "'hush'"], ["2", "'x'"]],
    );
}

#[test]
fn allow_pragmas_allows_only_the_pragmas_listed() {
    let authorizer = Authorizer::new().allow_pragmas(REPORTING_PRAGMAS);
    assert_allows(
        &authorizer,
        &[
            "PRAGMA table_info(t)",
            "PRAGMA TABLE_INFO(t)",
            "PRAGMA main.table_info(t)",
            "SELECT * FROM pragma_table_info('t')",
            "PRAGMA integrity_check",
            "PRAGMA compile_options",
        ],
    );
    assert_denies(
        &authorizer,
        &[
            "PRAGMA foreign_keys = OFF",
            "PRAGMA foreign_keys",
            "PRAGMA query_only = 0",
            "PRAGMA user_version",
            "SELECT * FROM pragma_user_version",
        ],
    );
    // Other actions are not its concern.
    assert_allows(&authorizer, READS);
    assert_writes(&authorizer, "DELETE FROM t", &[]);
}

#[test]
fn allow_pragma_reads_allows_the_pragmas_listed_only_without_a_value() {
    let authorizer = Authorizer::new().allow_pragma_reads(["user_version", "Foreign_Keys"]);
    assert_allows(
        &authorizer,
        &[
            "PRAGMA user_version",
            "PRAGMA Main.User_Version",
            "PRAGMA foreign_keys",
        ],
    );
    assert_denies(
        &authorizer,
        &[
            "PRAGMA user_version = 5",
            "PRAGMA foreign_keys = OFF",
            "PRAGMA table_info(t)",
        ],
    );
}

#[test]
fn the_pragma_lists_add_up() {
    let authorizer = Authorizer::new()
        .allow_pragmas(["table_info"])
        .allow_pragma_reads(["user_version"])
        .allow_pragmas(["index_list"]);
    assert_allows(
        &authorizer,
        &[
            "PRAGMA table_info(t)",
            "PRAGMA index_list(t)",
            "PRAGMA user_version",
        ],
    );
    assert_denies(
        &authorizer,
        &["PRAGMA user_version = 5", "PRAGMA foreign_keys"],
    );
}

#[test]
fn an_empty_allow_list_allows_none() {
    let none = || std::iter::empty::<&str>();
    assert_denies(
        &Authorizer::new().allow_pragmas(none()),
        &["PRAGMA table_info(t)"],
    );
    assert_denies(
        &Authorizer::new().allow_pragma_reads(none()),
        &["PRAGMA user_version"],
    );
    assert_denies(
        &Authorizer::new().allow_functions(none()),
        &["SELECT abs(-1)"],
    );
}

#[test]
fn allow_functions_allows_only_the_functions_listed() {
    let authorizer = Authorizer::new()
        .allow_functions(["abs"])
        .allow_functions(["LENGTH"]);
    assert_allows(
        &authorizer,
        &[
            "SELECT abs(-1)",
            "SELECT ABS(-1), length('a')",
            "SELECT x FROM t",
        ],
    );
    assert_denies(
        &authorizer,
        &[
            "SELECT upper('a')",
            "SELECT abs(-1), upper('a')",
            // Operators SQLite implements as functions.
            "SELECT 'a' LIKE 'a'",
            "SELECT 'a' GLOB 'a'",
            // Aggregates, and a function in a view's body.
            "SELECT count(*) FROM t",
            "CREATE TEMP VIEW tv AS SELECT upper(secret) FROM t; SELECT * FROM tv",
        ],
    );
}

#[test]
fn deny_functions_denies_the_functions_listed() {
    let authorizer = Authorizer::new().deny_functions(["upper"]);
    assert_allows(&authorizer, &["SELECT abs(-1)", "SELECT lower('A')"]);
    assert_denies(&authorizer, &["SELECT upper('a')", "SELECT UPPER('a')"]);
    // A denial outweighs an allowance.
    let both = Authorizer::new()
        .allow_functions(["upper"])
        .deny_functions(["upper"]);
    assert_denies(&both, &["SELECT upper('a')"]);
}

#[test]
fn a_rule_stands_with_the_built_in_ones() {
    // Reading `t` only through the view over it.
    let through_views = |authorizer: Authorizer| {
        authorizer.rule(|ctx| match ctx.action {
            AuthAction::Read { table_name, .. }
                if table_name.eq_ignore_ascii_case("t") && ctx.accessor.is_none() =>
            {
                Authorization::Deny
            }
            _ => Authorization::Allow,
        })
    };
    let authorizer = through_views(Authorizer::new());
    assert_allows(&authorizer, &["SELECT * FROM v"]);
    assert_denies(&authorizer, &["SELECT * FROM t"]);
    // A rule allowing cannot outweigh a built-in denial.
    let allowing = Authorizer::new()
        .deny_writes()
        .rule(|_| Authorization::Allow);
    assert_denies(&allowing, &["INSERT INTO t VALUES (2, 'x')"]);
    // Nor a built-in allowance a rule's denial.
    let combined = through_views(Authorizer::new().deny_writes());
    assert_denies(&combined, &["SELECT * FROM t", "DELETE FROM t"]);
    assert_allows(&combined, &["SELECT * FROM v"]);
}

/// A rule judging every read of `secret` as `judgement`.
fn on_secret(judgement: Authorization) -> impl Fn(AuthContext<'_>) -> Authorization {
    move |ctx| match ctx.action {
        AuthAction::Read {
            column_name: "secret",
            ..
        } => judgement,
        _ => Authorization::Allow,
    }
}

/// Assert `authorizer` reads `secret` as Null wherever a statement reads it.
fn assert_reads_secret_as_null(authorizer: &Authorizer) {
    for sql in [
        "INSERT INTO copy SELECT x, secret FROM t",
        "INSERT INTO copy SELECT x, SECRET FROM t WHERE true",
        "INSERT INTO copy SELECT x, secret FROM v",
    ] {
        let fixture = Fixture::new();
        app_value(&script(sql), fixture.factory(authorizer));
        assert_eq!(
            fixture.read("SELECT * FROM copy"),
            [["1", "NULL"]],
            "{authorizer:?}: {sql:?} read the column as Null"
        );
    }
}

#[test]
fn a_rule_may_hide_what_it_judges() {
    assert_reads_secret_as_null(&Authorizer::new().rule(on_secret(Authorization::Ignore)));
}

#[test]
fn the_strictest_judgement_stands_whatever_the_order() {
    let ignore = || on_secret(Authorization::Ignore);
    let deny = || on_secret(Authorization::Deny);
    let allow = || |_: AuthContext<'_>| Authorization::Allow;
    // Ignoring outweighs allowing.
    assert_reads_secret_as_null(&Authorizer::new().rule(ignore()).rule(allow()));
    assert_reads_secret_as_null(&Authorizer::new().rule(allow()).rule(ignore()));
    // Denying outweighs ignoring.
    assert_denies(
        &Authorizer::new().rule(ignore()).rule(deny()),
        &["SELECT secret FROM t"],
    );
    assert_denies(
        &Authorizer::new().rule(deny()).rule(ignore()),
        &["SELECT secret FROM t"],
    );
}

/// SQLite's limit, which [`Authorizer::rule`] documents: copying whole rows
/// between tables of the same shape reads no column.
#[test]
fn a_whole_row_copy_is_judged_only_as_an_insert() {
    let ignoring = Authorizer::new().rule(|ctx| match ctx.action {
        AuthAction::Read { .. } => Authorization::Ignore,
        _ => Authorization::Allow,
    });
    let fixture = Fixture::new();
    app_value(
        &script("INSERT INTO copy SELECT * FROM t"),
        fixture.factory(&ignoring),
    );
    assert_eq!(
        fixture.read("SELECT * FROM copy"),
        [["1", "'hush'"]],
        "no read was judged"
    );
    let no_inserts = Authorizer::new().rule(|ctx| match ctx.action {
        AuthAction::Insert { .. } => Authorization::Deny,
        _ => Authorization::Allow,
    });
    assert_denies(&no_inserts, &["INSERT INTO copy SELECT * FROM t"]);
}

#[test]
fn every_denial_is_worded_alike() {
    // SQLite words a denied column read as naming the column, and a denied
    // function under another code, naming the function.
    assert_denies(
        &Authorizer::new().rule(on_secret(Authorization::Deny)),
        &["SELECT secret FROM t"],
    );
    assert_denies(
        &Authorizer::new().deny_functions(["upper"]),
        &["SELECT upper('a')"],
    );
}

#[test]
fn a_failure_after_a_denial_keeps_its_own_wording() {
    let authorizer = Authorizer::new().deny_functions(["upper"]);
    let fixture = Fixture::new();
    let source = r"
        def db = import('app').open_db()
        [
            try_call(db.script, ['SELECT upper(1)']).error,
            try_call(db.script, ['SELECT * FROM missing']).error,
        ]
    ";
    assert_eq!(
        app_value(source, fixture.factory(&authorizer)),
        frost(&format!(
            "[{}, 'Function db.script failed: no such table: missing (SQLITE_ERROR)']",
            string_literal(DENIED)
        ))
    );
}

#[test]
fn a_denial_sqlite_tolerates_colours_no_later_failure() {
    // Connecting to a full-text table runs a pragma the allow-list denies,
    // which the module tolerates.
    let authorizer = Authorizer::new().allow_pragmas(REPORTING_PRAGMAS);
    let fixture = Fixture::new();
    fixture.set_up("CREATE VIRTUAL TABLE f USING fts4(body)");
    let source = r"
        def db = import('app').open_db()
        [
            db.script('SELECT count(*) FROM f'),
            try_call(db.script, ['SELECT * FROM f; SELECT * FROM missing']).error,
        ]
    ";
    assert_eq!(
        app_value(source, fixture.factory(&authorizer)),
        frost("[0, 'Function db.script failed: no such table: missing (SQLITE_ERROR)']")
    );
}

#[test]
fn a_panicking_rule_fails_the_statement() {
    let authorizer = Authorizer::new().rule(|ctx| match ctx.action {
        AuthAction::Delete { .. } => panic!("a rule with a bug"),
        _ => Authorization::Allow,
    });
    let fixture = Fixture::new();
    assert_eq!(
        app_raises(&script("DELETE FROM t"), fixture.factory(&authorizer)),
        DENIED
    );
    assert_eq!(fixture.read("SELECT count(*) FROM t"), [["1"]]);
}

/// A host's own authorizer, set on the connection with rusqlite: it denies
/// deletes, and reads of `secret`, and `upper`.
fn hosts_own(ctx: AuthContext<'_>) -> Authorization {
    match ctx.action {
        AuthAction::Delete { .. }
        | AuthAction::Read {
            column_name: "secret",
            ..
        }
        | AuthAction::Function {
            function_name: "upper",
        } => Authorization::Deny,
        _ => Authorization::Allow,
    }
}

#[test]
fn an_authorizer_replaces_the_connections_own() {
    let fixture = Fixture::new();
    let path = fixture.path();
    let factory = move |authorized: bool| {
        let path = path.clone();
        connection_factory("app.open_db", move |_| {
            let connection =
                Connection::open(&path).map_err(|err| sqlite_error("app.open_db", &err))?;
            connection
                .authorizer(Some(hosts_own))
                .map_err(|err| sqlite_error("app.open_db", &err))?;
            let connection = ScriptConnection::new(connection);
            Ok(if authorized {
                connection.with_authorizer(Authorizer::new().deny_attach())
            } else {
                connection
            })
        })
    };
    assert_eq!(
        app_raises(&script("DELETE FROM t"), factory(false)),
        DENIED,
        "without `with_authorizer`, the host's own stands"
    );
    assert_eq!(
        app_value(&script("DELETE FROM t"), factory(true)),
        frost("1"),
        "with it, the extension's replaces the host's"
    );
}

#[test]
fn a_hosts_own_denials_are_worded_by_their_code() {
    let fixture = Fixture::new();
    let path = fixture.path();
    let factory = connection_factory("app.open_db", move |_| {
        let connection =
            Connection::open(&path).map_err(|err| sqlite_error("app.open_db", &err))?;
        connection
            .authorizer(Some(hosts_own))
            .map_err(|err| sqlite_error("app.open_db", &err))?;
        Ok(ScriptConnection::new(connection))
    });
    let source = r"
        def db = import('app').open_db()
        [
            try_call(db.script, ['SELECT secret FROM t']).error,
            try_call(db.script, ['SELECT upper(1)']).error,
        ]
    ";
    // SQLite reports a denied read as `SQLITE_AUTH`, but a denied function as
    // `SQLITE_ERROR`, which only the extension's own authorizer confirms.
    assert_eq!(
        app_value(source, factory),
        frost(&format!(
            "[{}, 'Function db.script failed: not authorized to use function: upper (SQLITE_ERROR)']",
            string_literal(DENIED)
        ))
    );
}

/// The state of a pooled connection as it returned to its pool: whether a
/// delete succeeded on it, whether it could write, and whether it had no
/// transaction open.
type Released = Arc<Mutex<Option<(bool, bool, bool)>>>;

/// A factory of pooled in-memory connections, recording their release in
/// `released`; `expose` makes each a [`ScriptConnection`].
fn pooled_factory(
    released: &Released,
    expose: impl Fn(Pooled) -> ScriptConnection + Send + Sync + 'static,
) -> Value {
    let released = Arc::clone(released);
    connection_factory("app.open_db", move |_| {
        let connection = memory_connection()?;
        connection
            .authorizer(Some(hosts_own))
            .map_err(|err| sqlite_error("app.open_db", &err))?;
        let released = Arc::clone(&released);
        Ok(expose(Pooled::new(connection, move |connection| {
            let deletes = connection
                .execute_batch("CREATE TEMP TABLE r (x); DELETE FROM r")
                .is_ok();
            let writes = connection
                .execute_batch("CREATE TABLE after_release (x)")
                .is_ok();
            *released.lock().expect("no test panicked") =
                Some((deletes, writes, connection.is_autocommit()));
        })))
    })
}

#[test]
fn a_pooled_connection_returns_to_its_pool_without_the_authorizer() {
    // The authorizer denies writing, and even rolling back, which the
    // release must do all the same.
    let authorizer = Authorizer::new()
        .deny_writes()
        .rule(|ctx| match ctx.action {
            AuthAction::Transaction {
                operation: TransactionOperation::Rollback,
            } => Authorization::Deny,
            _ => Authorization::Allow,
        });
    let released = Released::default();
    let factory = pooled_factory(&released, move |pooled| {
        ScriptConnection::pooled(pooled).with_authorizer(authorizer.clone())
    });
    let source = r"
        def db = import('app').open_db()
        db.script('BEGIN; CREATE TEMP TABLE scratch (x)')
        [try_call(db.script, ['CREATE TABLE t (x)']).error, db.close()]
    ";
    assert_eq!(
        app_value(source, factory),
        frost(&format!("[{}, null]", string_literal(DENIED)))
    );
    assert_eq!(
        *released.lock().expect("no test panicked"),
        Some((true, true, true)),
        "released with no authorizer, the host's own replaced for good, and the transaction rolled back"
    );
}

#[test]
fn a_pooled_connection_keeps_its_own_authorizer_without_one_given() {
    let released = Released::default();
    let factory = pooled_factory(&released, ScriptConnection::pooled);
    let source = r"
        def db = import('app').open_db()
        db.script('BEGIN; CREATE TABLE t (x)')
        db.close()
    ";
    app_value(source, factory);
    assert_eq!(
        *released.lock().expect("no test panicked"),
        Some((false, true, true)),
        "released with the host's own authorizer, still denying deletes, and the transaction rolled back"
    );
}

#[test]
fn one_authorizer_serves_many_connections() {
    let authorizer = Authorizer::new().deny_writes();
    let fixture = Fixture::new();
    let source = r"
        def app = import('app')
        map range(3) with fn i -> try_call(app.open_db().script, ['DELETE FROM t']).error
    ";
    assert_eq!(
        app_value(source, fixture.factory(&authorizer)),
        frost(&format!("[{0}, {0}, {0}]", string_literal(DENIED)))
    );
}

#[test]
fn the_reporting_pragmas_only_report() {
    // Each runs under `deny_writes`, which denies a pragma setting a value,
    // with an argument where it takes one.
    let authorizer = Authorizer::new().deny_writes();
    let fixture = Fixture::new();
    for pragma in REPORTING_PRAGMAS {
        for sql in [format!("PRAGMA {pragma}"), format!("PRAGMA {pragma}(t)")] {
            let before = fixture.snapshot();
            // Some take no argument, and others another kind than a table;
            // only that none is denied, and none changes anything, matters.
            let result = run_app(
                &script(&sql),
                vec![("open_db", fixture.factory(&authorizer))],
            );
            if let Err(error) = result {
                assert_ne!(error.message(), DENIED, "{sql:?} is not denied");
            }
            assert_eq!(fixture.snapshot(), before, "{sql:?} changed nothing");
        }
    }
}
