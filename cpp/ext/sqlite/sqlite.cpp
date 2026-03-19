#include <frost/extensions-common.hpp>

#include <frost/builtin.hpp>
#include <frost/value.hpp>

#include <sqlite3.h>

#include <expected>

namespace frst
{
namespace sqlite
{

// Value_Ptr database_to_closuremap(const std::shared_ptr<Database>& db)
// {
//     STRINGS(execute);
//     return Value::create(
//         Value::trusted,
//         Map{{strings.execute, system_closure(1, 1, [db](builtin_args_t args)
//         {
//                  REQUIRE_ARGS("database.execute", TYPES(String));

//                  return Value::create(db->execute(GET(0, String)));
//              })}});
// }

// std::expected<std::shared_ptr<Database>, std::string> try_open_db(
//     const String& filename, int mode)
// {
//     try
//     {
//         return std::make_shared<Database>(filename, mode);
//     }
//     catch (const std::exception& e)
//     {
//         return std::unexpected{e.what()};
//     }
// }

// BUILTIN(open)
// {
//     REQUIRE_ARGS("sqlite.open", TYPES(String));
//     auto open_result = try_open_db(GET(0, String), SQLite::OPEN_READWRITE
//                                                        |
//                                                        SQLite::OPEN_CREATE);
//     if (not open_result)
//         return Value::create(std::move(open_result).error());

//     return database_to_closuremap(std::move(open_result).value());
// }

} // namespace sqlite

DECLARE_EXTENSION(sqlite)
{
    using namespace sqlite;
    CREATE_EXTENSION(
        {
            "version"_s,
            Value::create(String{sqlite3_version}),
        },
        // ENTRY(open, 1),
    );
}
} // namespace frst
