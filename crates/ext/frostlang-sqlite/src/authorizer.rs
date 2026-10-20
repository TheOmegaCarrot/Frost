//! Authorizers: what SQL a connection's scripts may run, as SQLite checks it.

use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

use rusqlite::ffi;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};

/// The pragmas that only report, never change anything, whatever their
/// argument: for [`Authorizer::allow_pragmas`].
///
/// They describe the schema, the library, and the database's integrity and
/// size. Some, such as `integrity_check`, can take long on a large database,
/// and some, such as `compile_options`, tell a script how the library was
/// built.
pub const REPORTING_PRAGMAS: &[&str] = &[
    "collation_list",
    "compile_options",
    "data_version",
    "database_list",
    "foreign_key_check",
    "foreign_key_list",
    "freelist_count",
    "function_list",
    "index_info",
    "index_list",
    "index_xinfo",
    "integrity_check",
    "module_list",
    "page_count",
    "pragma_list",
    "quick_check",
    "table_info",
    "table_list",
    "table_xinfo",
];

/// The pragmas that change a database file without taking an argument.
const WRITING_PRAGMAS: &[&str] = &["incremental_vacuum", "optimize", "wal_checkpoint"];

/// A host's own rule; see [`Authorizer::rule`].
type Rule = Arc<dyn Fn(AuthContext<'_>) -> Authorization + Send + Sync>;

/// What scripts may do on a connection, as SQLite checks each action a
/// statement takes, including the actions of the triggers and views it uses.
/// A [`ScriptConnection`](crate::ScriptConnection) takes one through
/// [`with_authorizer`](crate::ScriptConnection::with_authorizer).
///
/// SQLite judges most actions as it prepares a statement, before any of it
/// runs, but some as the statement runs: those of `VACUUM`, say, and of the
/// SQL that virtual tables run. Each action of SQL that SQLite or a virtual
/// table runs for a statement is judged as the statement's own.
///
/// An authorizer starts out allowing everything, and each method adds a rule.
/// Every rule judges every action, and the strictest judgement stands:
/// `Deny` over `Ignore` over `Allow`. A denied action fails its statement as
/// `not authorized (SQLITE_AUTH)`.
///
/// ```
/// use frostlang_sqlite::{Authorizer, REPORTING_PRAGMAS};
///
/// let reader = Authorizer::new()
///     .deny_writes()
///     .deny_attach()
///     .allow_pragmas(REPORTING_PRAGMAS);
/// ```
///
/// An authorizer guards against mistakes, such as a script writing where it
/// should only read, not against a script built to escape it. Opening the
/// connection with
/// [`SQLITE_OPEN_READ_ONLY`](rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) is
/// the stronger guarantee against writing the database file, as SQLite itself
/// enforces it, but it does not stop `VACUUM INTO` writing a new file:
/// [`deny_attach`](Self::deny_attach) does.
#[derive(Clone, Default)]
pub struct Authorizer {
    deny_writes: bool,
    deny_attach: bool,
    /// `None` while every pragma is allowed.
    pragmas: Option<Pragmas>,
    /// The functions allowed; `None` while every function is.
    allowed_functions: Option<Names>,
    denied_functions: Names,
    rules: Vec<Rule>,
}

/// The pragmas an [`Authorizer`] allows.
#[derive(Clone, Debug, Default)]
struct Pragmas {
    /// Allowed with any argument.
    any: Names,
    /// Allowed only with no argument.
    reads: Names,
}

/// SQL names, lowercase, as SQL matches them regardless of case.
type Names = BTreeSet<String>;

/// `names`, lowercase, for [`Names`].
fn lowercased(names: impl IntoIterator<Item = impl AsRef<str>>) -> impl Iterator<Item = String> {
    names
        .into_iter()
        .map(|name| name.as_ref().to_ascii_lowercase())
}

/// Whether `names` holds `name`, regardless of case.
fn holds(names: &Names, name: &str) -> bool {
    names.contains(&name.to_ascii_lowercase())
}

impl Authorizer {
    /// An authorizer allowing everything.
    pub fn new() -> Self {
        Self::default()
    }

    /// Denies changing any database but the `temp` one: its data and its
    /// schema, as through `VACUUM`, `REINDEX`, or `ANALYZE`. Reading,
    /// transactions, and temporary tables, views, indexes, and triggers on
    /// them stay allowed.
    ///
    /// It also denies every pragma given an argument, whatever its database,
    /// but those of [`REPORTING_PRAGMAS`], whose argument is what to report on:
    /// connection settings too, such as `PRAGMA foreign_keys = OFF`. It denies
    /// the pragmas that write with no argument, such as `PRAGMA optimize`.
    ///
    /// It does not deny attaching another database file, which lets a script
    /// read any database the process can, and create an empty file where none
    /// was: even a denied `VACUUM INTO` leaves one.
    /// [`deny_attach`](Self::deny_attach) denies both.
    ///
    /// It also denies reading a virtual table whose module writes as it
    /// connects, such as an R*Tree. Some older versions of SQLite, 3.34 among
    /// them, report every use of a virtual table as a write to the schema, so
    /// with those it denies them all, table-valued functions such as
    /// `json_each` and `pragma_table_info` included.
    #[must_use]
    pub fn deny_writes(self) -> Self {
        Self {
            deny_writes: true,
            ..self
        }
    }

    /// Denies reaching other database files: `ATTACH`, `DETACH`, and
    /// `VACUUM INTO`, which writes a copy of the database to a new file.
    /// Only attaching a private temporary database, as `ATTACH '' AS name`
    /// does, stays allowed; `:memory:` and URIs are denied, as is detaching any
    /// database.
    #[must_use]
    pub fn deny_attach(self) -> Self {
        Self {
            deny_attach: true,
            ..self
        }
    }

    /// Allows the pragmas `names`, with any argument, and denies every pragma
    /// that neither this nor [`allow_pragma_reads`](Self::allow_pragma_reads)
    /// allows, so an empty list allows none. [`REPORTING_PRAGMAS`] lists the
    /// pragmas no argument makes change anything.
    ///
    /// Pragmas that SQLite or a virtual table runs for a statement are judged
    /// too: creating an R*Tree runs `page_size`, and querying an FTS5 table
    /// runs `data_version`.
    #[must_use]
    pub fn allow_pragmas(mut self, names: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        self.pragmas
            .get_or_insert_default()
            .any
            .extend(lowercased(names));
        self
    }

    /// Allows the pragmas `names` with no argument, which reads a setting,
    /// as `PRAGMA user_version` does, but not with one, which sets it, as
    /// `PRAGMA user_version = 2` does. Denies every pragma that neither this
    /// nor [`allow_pragmas`](Self::allow_pragmas) allows, so an empty list
    /// allows none.
    #[must_use]
    pub fn allow_pragma_reads(mut self, names: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        self.pragmas
            .get_or_insert_default()
            .reads
            .extend(lowercased(names));
        self
    }

    /// Allows only the SQL functions `names`, together with those any other
    /// call of this allows, so an empty list allows none.
    ///
    /// A function is named as SQLite registers it: its aliases, such as
    /// `substring` for `substr`, and the operators SQLite implements as
    /// functions, such as `like`, `glob`, `match`, `->`, and `->>`, each go by
    /// their own name. Aggregate and window functions are judged alike.
    /// Table-valued functions, such as `json_each`, are not judged, nor are
    /// the expressions a schema stores, such as a column's default, when a
    /// statement uses them.
    ///
    /// Functions that SQLite or a virtual table calls for a statement are
    /// judged too: `VACUUM` calls `coalesce`, `ALTER TABLE ... RENAME` calls
    /// SQLite's own `sqlite_rename_table`, and full-text queries call `match`.
    #[must_use]
    pub fn allow_functions(mut self, names: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        self.allowed_functions
            .get_or_insert_default()
            .extend(lowercased(names));
        self
    }

    /// Denies the SQL functions `names`, named as for
    /// [`allow_functions`](Self::allow_functions). A function SQLite adds
    /// later, or a host registers, stays allowed, so `allow_functions` is the
    /// surer guard.
    #[must_use]
    pub fn deny_functions(mut self, names: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        self.denied_functions.extend(lowercased(names));
        self
    }

    /// Adds `rule`, the host's own judgement of each action, which stands
    /// with every other rule. SQLite's documentation of
    /// [`sqlite3_set_authorizer`](https://www.sqlite.org/c3ref/set_authorizer.html)
    /// gives each action's arguments, and what `Ignore` means for it: a read
    /// of a column gives Null, but some actions, such as a `DELETE`, go ahead
    /// regardless.
    ///
    /// ```
    /// use frostlang_sqlite::Authorizer;
    /// use frostlang_sqlite::rusqlite::hooks::{AuthAction, Authorization};
    ///
    /// // Scripts insert only into `main.events`.
    /// let authorizer = Authorizer::new().rule(|ctx| match ctx.action {
    ///     AuthAction::Insert { table_name }
    ///         if !(table_name.eq_ignore_ascii_case("events")
    ///             && ctx.database_name == Some("main")) =>
    ///     {
    ///         Authorization::Deny
    ///     }
    ///     _ => Authorization::Allow,
    /// });
    /// ```
    ///
    /// A rule panicking fails the statement.
    ///
    /// A rule judging reads cannot keep a table's contents from a script that
    /// may insert into another table of the same shape. `INSERT INTO b SELECT
    /// * FROM a`, with no `WHERE` clause, copies whole rows without SQLite
    /// asking the authorizer about any column of `a`, so `Deny` or `Ignore`
    /// for a `Read` never applies, though the `Insert` is judged.
    #[must_use]
    pub fn rule(
        mut self,
        rule: impl Fn(AuthContext<'_>) -> Authorization + Send + Sync + 'static,
    ) -> Self {
        self.rules.push(Arc::new(rule));
        self
    }

    /// The judgement of `ctx`'s action: the strictest of every rule's.
    pub(crate) fn judge(&self, ctx: AuthContext<'_>) -> Authorization {
        let built_in = [
            self.judge_write(ctx),
            self.judge_attach(ctx),
            self.judge_pragma(ctx),
            self.judge_function(ctx),
        ];
        let own = self.rules.iter().map(|rule| rule(ctx));
        built_in
            .into_iter()
            .chain(own)
            .max_by_key(|judgement| strictness(*judgement))
            .unwrap_or(Authorization::Allow)
    }

    fn judge_write(&self, ctx: AuthContext<'_>) -> Authorization {
        if !self.deny_writes {
            return Authorization::Allow;
        }
        let in_temp = ctx.database_name.is_some_and(is_temp);
        let writes = match ctx.action {
            AuthAction::Insert { .. }
            | AuthAction::Update { .. }
            | AuthAction::Delete { .. }
            | AuthAction::CreateIndex { .. }
            | AuthAction::CreateTable { .. }
            | AuthAction::CreateTrigger { .. }
            | AuthAction::CreateView { .. }
            | AuthAction::DropIndex { .. }
            | AuthAction::DropTable { .. }
            | AuthAction::DropTrigger { .. }
            | AuthAction::DropView { .. }
            | AuthAction::CreateVtable { .. }
            | AuthAction::DropVtable { .. }
            | AuthAction::Reindex { .. }
            | AuthAction::Analyze { .. } => !in_temp,
            // SQLite names the database altered in the action, not the context.
            AuthAction::AlterTable { database_name, .. } => !is_temp(database_name),
            // A pragma's argument is either a setting or what to report on,
            // which only the pragma's name tells apart.
            AuthAction::Pragma {
                pragma_name,
                pragma_value,
            } => {
                let name = pragma_name.to_ascii_lowercase();
                WRITING_PRAGMAS.contains(&name.as_str())
                    || (pragma_value.is_some() && !REPORTING_PRAGMAS.contains(&name.as_str()))
            }
            AuthAction::Select
            | AuthAction::Read { .. }
            | AuthAction::Function { .. }
            | AuthAction::Recursive
            | AuthAction::Transaction { .. }
            | AuthAction::Savepoint { .. }
            | AuthAction::Attach { .. }
            | AuthAction::Detach { .. }
            | AuthAction::CreateTempIndex { .. }
            | AuthAction::CreateTempTable { .. }
            | AuthAction::CreateTempTrigger { .. }
            | AuthAction::CreateTempView { .. }
            | AuthAction::DropTempIndex { .. }
            | AuthAction::DropTempTable { .. }
            | AuthAction::DropTempTrigger { .. }
            | AuthAction::DropTempView { .. } => false,
            // Attaching is `deny_attach`'s to judge, however it arrives.
            AuthAction::Unknown { code, .. } if is_attach(code) => false,
            // An action this crate does not know, taken as a write.
            _ => true,
        };
        deny_if(writes)
    }

    fn judge_attach(&self, ctx: AuthContext<'_>) -> Authorization {
        let reaches_a_file = match ctx.action {
            AuthAction::Attach { filename } => !filename.is_empty(),
            AuthAction::Detach { .. } => true,
            // An attach whose file name is not a string literal, which SQLite
            // judges with no name.
            AuthAction::Unknown { code, .. } => is_attach(code),
            _ => false,
        };
        deny_if(self.deny_attach && reaches_a_file)
    }

    fn judge_pragma(&self, ctx: AuthContext<'_>) -> Authorization {
        let (
            AuthAction::Pragma {
                pragma_name,
                pragma_value,
            },
            Some(pragmas),
        ) = (ctx.action, &self.pragmas)
        else {
            return Authorization::Allow;
        };
        let allowed = holds(&pragmas.any, pragma_name)
            || (pragma_value.is_none() && holds(&pragmas.reads, pragma_name));
        deny_if(!allowed)
    }

    fn judge_function(&self, ctx: AuthContext<'_>) -> Authorization {
        let AuthAction::Function { function_name } = ctx.action else {
            return Authorization::Allow;
        };
        let allowed = self
            .allowed_functions
            .as_ref()
            .is_none_or(|allowed| holds(allowed, function_name));
        deny_if(!allowed || holds(&self.denied_functions, function_name))
    }
}

impl fmt::Debug for Authorizer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Authorizer")
            .field("deny_writes", &self.deny_writes)
            .field("deny_attach", &self.deny_attach)
            .field("pragmas", &self.pragmas)
            .field("allowed_functions", &self.allowed_functions)
            .field("denied_functions", &self.denied_functions)
            .field("rules", &self.rules.len())
            .finish()
    }
}

/// Whether `database` is the `temp` one.
fn is_temp(database: &str) -> bool {
    database.eq_ignore_ascii_case("temp")
}

/// Whether `code` is SQLite's action code for attaching or detaching.
fn is_attach(code: i32) -> bool {
    code == ffi::SQLITE_ATTACH || code == ffi::SQLITE_DETACH
}

fn deny_if(denied: bool) -> Authorization {
    if denied {
        Authorization::Deny
    } else {
        Authorization::Allow
    }
}

/// How strict `judgement` is, for choosing the strictest of several. A
/// judgement this crate does not know is taken as a denial.
fn strictness(judgement: Authorization) -> u8 {
    match judgement {
        Authorization::Allow => 0,
        Authorization::Ignore => 1,
        _ => 2,
    }
}
