#include <frost/extensions-common.hpp>

#include <frost/builtin.hpp>
#include <frost/value.hpp>

#include <sqlite3.h>

#include "connection.hpp"

namespace frst
{
namespace sqlite
{

namespace
{
Array extract_bindings(builtin_args_t args, std::size_t idx)
{
    return idx < args.size() ? args.at(idx)->raw_get<Array>() : Array{};
}

struct Bindings_And_Callback
{
    Array bindings;
    const Function& callback;
};

// For (sql, [bindings], callback) signatures: resolves the optional
// middle argument based on arity.
Bindings_And_Callback extract_bindings_and_callback(builtin_args_t args)
{
    if (args.size() == 3)
        return {GET(1, Array), GET(2, Function)};
    return {{}, GET(1, Function)};
}

// Build the data-access closures shared by both db and tx maps.
// The guard is called before each operation: the db guard rejects calls
// during an active transaction; the tx guard rejects calls after the
// transaction has ended.
Map make_data_closures(std::shared_ptr<Connection> conn, std::string prefix,
                       std::function<void()> guard)
{
    STRINGS(exec, query, each, collect, script);
    auto px = [&](const char* method) { return prefix + "." + method; };

    return Map{
        {strings.exec,
         system_closure(
             1, 2,
             [conn, guard, name = px("exec")](builtin_args_t args) {
                 guard();
                 REQUIRE_ARGS(name, PARAM("SQL", TYPES(String)),
                              OPTIONAL(PARAM("bindings", TYPES(Array))));

                 return Value::create(
                     conn->exec(GET(0, String), extract_bindings(args, 1)));
             })},
        {strings.script,
         system_closure(1, 1,
                        [conn, guard, name = px("script")](builtin_args_t args) {
                            guard();
                            REQUIRE_ARGS(name, PARAM("SQL", TYPES(String)));
                            return Value::create(
                                conn->script(GET(0, String)));
                        })},
        {strings.query,
         system_closure(
             1, 2,
             [conn, guard, name = px("query")](builtin_args_t args) {
                 guard();
                 REQUIRE_ARGS(name, PARAM("SQL", TYPES(String)),
                              OPTIONAL(PARAM("bindings", TYPES(Array))));

                 Array rows;
                 conn->for_each_row(GET(0, String),
                                    extract_bindings(args, 1),
                                    [&](Value_Ptr row) {
                                        rows.push_back(std::move(row));
                                    });
                 return Value::create(std::move(rows));
             })},
        {strings.each,
         system_closure(
             2, 3,
             [conn, guard, name = px("each")](builtin_args_t args) {
                 guard();
                 if (args.size() == 3)
                     REQUIRE_ARGS(name, PARAM("SQL", TYPES(String)),
                                  PARAM("bindings", TYPES(Array)),
                                  PARAM("callback", TYPES(Function)));
                 else
                     REQUIRE_ARGS(name, PARAM("SQL", TYPES(String)),
                                  PARAM("callback", TYPES(Function)));

                 auto [bindings, callback] =
                     extract_bindings_and_callback(args);

                 conn->for_each_row(GET(0, String), bindings,
                                    [&](Value_Ptr row) {
                                        callback->call({row});
                                    });
                 return Value::null();
             })},
        {strings.collect,
         system_closure(
             2, 3,
             [conn, guard, name = px("collect")](builtin_args_t args) {
                 guard();
                 if (args.size() == 3)
                     REQUIRE_ARGS(name, PARAM("SQL", TYPES(String)),
                                  PARAM("bindings", TYPES(Array)),
                                  PARAM("callback", TYPES(Function)));
                 else
                     REQUIRE_ARGS(name, PARAM("SQL", TYPES(String)),
                                  PARAM("callback", TYPES(Function)));

                 auto [bindings, callback] =
                     extract_bindings_and_callback(args);

                 Array results;
                 conn->for_each_row(
                     GET(0, String), bindings, [&](Value_Ptr row) {
                         results.push_back(callback->call({row}));
                     });
                 return Value::create(std::move(results));
             })},
    };
}
} // namespace

Value_Ptr database_to_closuremap(const std::shared_ptr<Connection>& conn)
{
    STRINGS(close, transaction);

    auto guard = [conn]() {
        if (conn->in_transaction())
            throw Frost_Recoverable_Error{
                "connection has an active transaction; "
                "use the transaction object instead"};
    };

    Map entries = make_data_closures(conn, "database", guard);

    entries.insert_or_assign(
        strings.close,
        system_closure(0, 0,
                       [conn](builtin_args_t) {
                           if (conn->in_transaction())
                               throw Frost_Recoverable_Error{
                                   "cannot close connection while a "
                                   "transaction is active"};
                           conn->close();
                           return Value::null();
                       }));

    entries.insert_or_assign(
        strings.transaction,
        system_closure(
            1, 1,
            [conn](builtin_args_t args) {
                REQUIRE_ARGS("database.transaction",
                             PARAM("callback", TYPES(Function)));

                auto& callback = GET(0, Function);

                if (conn->in_transaction())
                    throw Frost_Recoverable_Error{
                        "connection has an active transaction: "
                        "cannot start a nested transaction"};

                int before = conn->total_changes();
                conn->script("BEGIN");

                auto tx_guard = [conn]() {
                    if (!conn->in_transaction())
                        throw Frost_Recoverable_Error{
                            "transaction is no longer active"};
                };

                Map tx_entries =
                    make_data_closures(conn, "transaction", tx_guard);
                auto tx = Value::create(Value::trusted, std::move(tx_entries));

                try
                {
                    callback->call({tx});
                    conn->script("COMMIT");
                    return Value::create(
                        Int{conn->total_changes() - before});
                }
                catch (...)
                {
                    if (conn->in_transaction())
                        conn->script("ROLLBACK");
                    throw;
                }
            }));

    return Value::create(Value::trusted, std::move(entries));
}

std::shared_ptr<Connection> open_db(const String& filename, int mode)
{
    return Connection::create(filename, mode);
}

BUILTIN(open)
{
    REQUIRE_ARGS("sqlite.open", TYPES(String));
    auto conn =
        open_db(GET(0, String), SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE);

    return database_to_closuremap(std::move(conn));
}

BUILTIN(open_readonly)
{
    REQUIRE_ARGS("sqlite.open_readonly", TYPES(String));
    auto conn = open_db(GET(0, String), SQLITE_OPEN_READONLY);

    return database_to_closuremap(std::move(conn));
}

BUILTIN(open_memory)
{
    auto conn = open_db(":memory:", SQLITE_OPEN_READWRITE);

    return database_to_closuremap(std::move(conn));
}

} // namespace sqlite

DECLARE_EXTENSION(sqlite)
{
    using namespace sqlite;
    CREATE_EXTENSION(
        {
            "version"_s,
            Value::create(String{sqlite3_version}),
        },
        ENTRY(open, 1), ENTRY(open_readonly, 1), ENTRY(open_memory, 0), );
}
} // namespace frst
