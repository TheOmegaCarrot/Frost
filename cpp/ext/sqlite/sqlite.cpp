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
bool has_bindings(builtin_args_t args, std::size_t idx)
{
    return idx < args.size() && not args.at(idx)->is<Function>();
}

template <std::invocable<Value_Ptr> Row_Fn>
void for_each_row_dispatch(Connection& conn, const String& sql,
                           builtin_args_t args, std::size_t idx,
                           Row_Fn&& row_fn)
{
    if (not has_bindings(args, idx))
    {
        conn.for_each_row(sql, Array{}, std::forward<Row_Fn>(row_fn));
    }
    else if (args.at(idx)->is<Array>())
    {
        conn.for_each_row(sql, args.at(idx)->raw_get<Array>(),
                          std::forward<Row_Fn>(row_fn));
    }
    else
    {
        conn.for_each_row(sql, args.at(idx)->raw_get<Map>(),
                          std::forward<Row_Fn>(row_fn));
    }
}

// Frost-facing glue for the data-access methods shared by both the
// database and transaction closure maps.
struct Data_Methods
{
    std::shared_ptr<Connection> conn;
    std::function<void()> guard;
    std::string prefix;

    std::string name(const char* method) const
    {
        return prefix + "." + method;
    }

    Value_Ptr exec(builtin_args_t args)
    {
        guard();
        auto n = name("exec");
        REQUIRE_ARGS(n, PARAM("SQL", TYPES(String)),
                     OPTIONAL(PARAM("bindings", TYPES(Array, Map))));

        auto& sql = GET(0, String);
        if (not has_bindings(args, 1))
            return Value::create(conn->exec(sql, Array{}));
        if (args.at(1)->is<Array>())
            return Value::create(conn->exec(sql, args.at(1)->raw_get<Array>()));
        return Value::create(conn->exec(sql, args.at(1)->raw_get<Map>()));
    }

    Value_Ptr script(builtin_args_t args)
    {
        guard();
        auto n = name("script");
        REQUIRE_ARGS(n, PARAM("SQL", TYPES(String)));
        return Value::create(conn->script(GET(0, String)));
    }

    Value_Ptr query(builtin_args_t args)
    {
        guard();
        auto n = name("query");
        REQUIRE_ARGS(n, PARAM("SQL", TYPES(String)),
                     OPTIONAL(PARAM("bindings", TYPES(Array, Map))));

        Array rows;
        for_each_row_dispatch(*conn, GET(0, String), args, 1,
                              [&](Value_Ptr row) {
                                  rows.push_back(std::move(row));
                              });
        return Value::create(std::move(rows));
    }

    Value_Ptr each(builtin_args_t args)
    {
        guard();
        auto n = name("each");
        if (args.size() == 3)
            REQUIRE_ARGS(n, PARAM("SQL", TYPES(String)),
                         PARAM("bindings", TYPES(Array, Map)),
                         PARAM("callback", TYPES(Function)));
        else
            REQUIRE_ARGS(n, PARAM("SQL", TYPES(String)),
                         PARAM("callback", TYPES(Function)));

        auto& callback = args.back()->raw_get<Function>();
        for_each_row_dispatch(*conn, GET(0, String), args, 1,
                              [&](Value_Ptr row) {
                                  callback->call({row});
                              });
        return Value::null();
    }

    Value_Ptr collect(builtin_args_t args)
    {
        guard();
        auto n = name("collect");
        if (args.size() == 3)
            REQUIRE_ARGS(n, PARAM("SQL", TYPES(String)),
                         PARAM("bindings", TYPES(Array, Map)),
                         PARAM("callback", TYPES(Function)));
        else
            REQUIRE_ARGS(n, PARAM("SQL", TYPES(String)),
                         PARAM("callback", TYPES(Function)));

        auto& callback = args.back()->raw_get<Function>();
        Array results;
        for_each_row_dispatch(*conn, GET(0, String), args, 1,
                              [&](Value_Ptr row) {
                                  results.push_back(callback->call({row}));
                              });
        return Value::create(std::move(results));
    }

    Value_Ptr last_insert_rowid(builtin_args_t)
    {
        guard();
        return Value::create(conn->last_insert_rowid());
    }

    Map to_map()
    {
        STRINGS(exec, query, each, collect, script, last_insert_rowid);
        auto self = std::make_shared<Data_Methods>(std::move(*this));
        return Map{
            {strings.exec, system_closure([self](builtin_args_t args) {
                 return self->exec(args);
             })},
            {strings.script, system_closure([self](builtin_args_t args) {
                 return self->script(args);
             })},
            {strings.query, system_closure([self](builtin_args_t args) {
                 return self->query(args);
             })},
            {strings.each, system_closure([self](builtin_args_t args) {
                 return self->each(args);
             })},
            {strings.collect, system_closure([self](builtin_args_t args) {
                 return self->collect(args);
             })},
            {strings.last_insert_rowid,
             system_closure([self](builtin_args_t args) {
                 REQUIRE_NULLARY("database.last_insert_rowid");
                 return self->last_insert_rowid(args);
             })},
        };
    }
};
// Frost-facing glue for db-only methods (close, transaction, etc.)
struct Database_Methods
{
    std::shared_ptr<Connection> conn;

    Value_Ptr close(builtin_args_t args)
    {
        REQUIRE_NULLARY("database.close");
        if (conn->in_transaction())
            throw Frost_Recoverable_Error{"cannot close connection while a "
                                          "transaction is active"};
        conn->close();
        return Value::null();
    }

    Value_Ptr transaction(builtin_args_t args)
    {
        REQUIRE_ARGS("database.transaction",
                     PARAM("callback", TYPES(Function)));

        auto& callback = GET(0, Function);

        if (conn->in_transaction())
            throw Frost_Recoverable_Error{
                "connection has an active transaction: "
                "cannot start a nested transaction"};

        int before = conn->total_changes();
        conn->script("BEGIN");

        auto tx_guard = [c = conn]() {
            if (not c->in_transaction())
                throw Frost_Recoverable_Error{
                    "transaction is no longer active"};
        };

        Map tx_entries = Data_Methods{conn, tx_guard, "transaction"}.to_map();
        auto tx = Value::create(Value::trusted, std::move(tx_entries));

        try
        {
            callback->call({tx});
            conn->script("COMMIT");
            return Value::create(Int{conn->total_changes() - before});
        }
        catch (...)
        {
            if (conn->in_transaction())
                conn->script("ROLLBACK");
            throw;
        }
    }

    void merge_into(Map& entries)
    {
        STRINGS(close, transaction);
        auto self = std::make_shared<Database_Methods>(std::move(*this));
        entries.insert_or_assign(strings.close,
                                 system_closure([self](builtin_args_t args) {
                                     return self->close(args);
                                 }));
        entries.insert_or_assign(strings.transaction,
                                 system_closure([self](builtin_args_t args) {
                                     return self->transaction(args);
                                 }));
    }
};
} // namespace

Value_Ptr database_to_closuremap(const std::shared_ptr<Connection>& conn)
{
    auto guard = [conn]() {
        if (conn->in_transaction())
            throw Frost_Recoverable_Error{
                "connection has an active transaction: "
                "use the transaction object instead"};
    };

    Map entries = Data_Methods{conn, guard, "database"}.to_map();
    Database_Methods{conn}.merge_into(entries);

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
    REQUIRE_NULLARY("sqlite.open_memory");
    auto conn = open_db(":memory:", SQLITE_OPEN_READWRITE);

    return database_to_closuremap(std::move(conn));
}

} // namespace sqlite

REGISTER_EXTENSION(sqlite,
                   {
                       "version"_s,
                       Value::create(String{sqlite3_version}),
                   },
                   ENTRY(open), ENTRY(open_readonly), ENTRY(open_memory))

} // namespace frst
