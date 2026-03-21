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
} // namespace

Value_Ptr database_to_closuremap(const std::shared_ptr<Connection>& conn)
{
    STRINGS(exec, close, query, each, collect);
    return Value::create(
        Value::trusted,
        Map{
            {strings.exec,
             system_closure(1, 2,
                            [conn](builtin_args_t args) {
                                REQUIRE_ARGS(
                                    "database.exec",
                                    PARAM("SQL", TYPES(String)),
                                    OPTIONAL(PARAM("bindings", TYPES(Array))));

                                return Value::create(conn->exec(
                                    GET(0, String), extract_bindings(args, 1)));
                            })},
            {strings.close, system_closure(0, 0,
                                           [conn](builtin_args_t) {
                                               conn->close();
                                               return Value::null();
                                           })},
            {strings.query,
             system_closure(1, 2,
                            [conn](builtin_args_t args) {
                                REQUIRE_ARGS(
                                    "database.query",
                                    PARAM("SQL", TYPES(String)),
                                    OPTIONAL(PARAM("bindings", TYPES(Array))));

                                Array rows;
                                conn->for_each_row(
                                    GET(0, String), extract_bindings(args, 1),
                                    [&](Value_Ptr row) {
                                        rows.push_back(std::move(row));
                                    });
                                return Value::create(std::move(rows));
                            })},
            {strings.each,
             system_closure(
                 2, 3,
                 [conn](builtin_args_t args) {
                     if (args.size() == 3)
                         REQUIRE_ARGS("database.each",
                                      PARAM("SQL", TYPES(String)),
                                      PARAM("bindings", TYPES(Array)),
                                      PARAM("callback", TYPES(Function)));
                     else
                         REQUIRE_ARGS("database.each",
                                      PARAM("SQL", TYPES(String)),
                                      PARAM("callback", TYPES(Function)));

                     auto [bindings, callback] =
                         extract_bindings_and_callback(args);

                     conn->for_each_row(
                         GET(0, String), bindings,
                         [&](Value_Ptr row) { callback->call({row}); });
                     return Value::null();
                 })},
            {strings.collect,
             system_closure(
                 2, 3,
                 [conn](builtin_args_t args) {
                     if (args.size() == 3)
                         REQUIRE_ARGS("database.collect",
                                      PARAM("SQL", TYPES(String)),
                                      PARAM("bindings", TYPES(Array)),
                                      PARAM("callback", TYPES(Function)));
                     else
                         REQUIRE_ARGS("database.collect",
                                      PARAM("SQL", TYPES(String)),
                                      PARAM("callback", TYPES(Function)));

                     auto [bindings, callback] =
                         extract_bindings_and_callback(args);

                     Array results;
                     conn->for_each_row(
                         GET(0, String), bindings,
                         [&](Value_Ptr row) {
                             results.push_back(callback->call({row}));
                         });
                     return Value::create(std::move(results));
                 })},
        });
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

} // namespace sqlite

DECLARE_EXTENSION(sqlite)
{
    using namespace sqlite;
    CREATE_EXTENSION(
        {
            "version"_s,
            Value::create(String{sqlite3_version}),
        },
        ENTRY(open, 1), );
}
} // namespace frst
