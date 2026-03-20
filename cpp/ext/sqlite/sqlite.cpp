#include <frost/extensions-common.hpp>

#include <frost/builtin.hpp>
#include <frost/value.hpp>

#include <sqlite3.h>

#include "connection.hpp"

namespace frst
{
namespace sqlite
{

Value_Ptr database_to_closuremap(const std::shared_ptr<Connection>& conn)
{
    STRINGS(execute, close);
    return Value::create(
        Value::trusted,
        Map{
            {strings.execute,
             system_closure(1, 1,
                            [conn](builtin_args_t args) {
                                REQUIRE_ARGS("database.execute", TYPES(String));

                                return Value::create(
                                    conn->exec(GET(0, String)));
                            })},
            {strings.close, system_closure(0, 0,
                                           [conn](builtin_args_t args) {
                                               conn->close();
                                               return Value::null();
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
