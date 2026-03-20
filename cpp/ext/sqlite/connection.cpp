#include "connection.hpp"

#include <boost/scope_exit.hpp>

namespace frst::sqlite
{

std::shared_ptr<Connection> Connection::create(const String& filename,
                                               int openmode)
{
    auto conn = std::make_shared<Connection>(Restricted{});

    int rc = sqlite3_open_v2(filename.c_str(), &conn->conn_, openmode, nullptr);
    if (rc != SQLITE_OK)
    {
        std::string msg = sqlite3_errmsg(conn->conn_);
        conn->close();
        throw Frost_Recoverable_Error{msg};
    }

    return conn;
}

Value_Ptr Connection::query_positional(const String& sql, const Array& bindings)
{
    require_open();

    sqlite3_stmt* stmt = nullptr;
    BOOST_SCOPE_EXIT_ALL(&)
    {
        sqlite3_finalize(stmt);
    };

    int prepare_rc = sqlite3_prepare_v2(
        conn_, sql.c_str(), static_cast<int>(sql.size()), &stmt, nullptr);
    if (prepare_rc != SQLITE_OK)
    {
        std::string msg = sqlite3_errmsg(conn_);
        throw Frost_Recoverable_Error{msg};
    }

    int param_count = sqlite3_bind_parameter_count(stmt);
    if (param_count != static_cast<int>(bindings.size()))
    {
        throw Frost_Recoverable_Error{
            fmt::format("query expected {} binding{}, got {}", param_count,
                        param_count == 1 ? "" : "s", bindings.size())};
    }

    // bind the parameters (if any)
    for (const auto& [idx, val_ptr] : std::views::enumerate(bindings))
    {
        int pos = static_cast<int>(idx) + 1;
        val_ptr->visit(Overload{
            [&](const Null&) {
                sqlite3_bind_null(stmt, pos);
            },
            [&](const Int& v) {
                sqlite3_bind_int64(stmt, pos, v);
            },
            [&](const Float& v) {
                sqlite3_bind_double(stmt, pos, v);
            },
            [&](const Bool& v) {
                sqlite3_bind_int64(stmt, pos, v ? 1 : 0);
            },
            [&](const String& v) {
                sqlite3_bind_text(stmt, pos, v.c_str(),
                                  static_cast<int>(v.size()), SQLITE_TRANSIENT);
            },
            [&](const auto&) {
                throw Frost_Recoverable_Error{
                    "unsupported binding type at index " + std::to_string(idx)};
            },
        });
    }

    // collect the rows
    Array result_rows;
    int num_cols = sqlite3_column_count(stmt);
    while (true)
    {
        int rc = sqlite3_step(stmt);
        if (rc == SQLITE_DONE)
            break;
        if (rc != SQLITE_ROW)
        {
            std::string msg = sqlite3_errmsg(conn_);
            throw Frost_Recoverable_Error{msg};
        }

        Map row;
        // Collect one row
        for (int i = 0; i != num_cols; ++i)
        {
            auto col_name = Value::create(String{sqlite3_column_name(stmt, i)});
            switch (sqlite3_column_type(stmt, i))
            {
            case SQLITE_INTEGER:
                row.insert_or_assign(
                    col_name,
                    Value::create(Int{sqlite3_column_int64(stmt, i)}));
                break;
            case SQLITE_FLOAT:
                row.insert_or_assign(
                    col_name, Value::create(sqlite3_column_double(stmt, i)));
                break;
            case SQLITE_TEXT:
                row.insert_or_assign(
                    col_name,
                    Value::create(String{
                        reinterpret_cast<const char*>(
                            sqlite3_column_text(stmt, i)),
                        static_cast<std::size_t>(sqlite3_column_bytes(stmt, i)),
                    }));
                break;
            case SQLITE_BLOB:
                row.insert_or_assign(
                    col_name,
                    Value::create(String{
                        static_cast<const char*>(sqlite3_column_blob(stmt, i)),
                        static_cast<std::size_t>(sqlite3_column_bytes(stmt, i)),
                    }));
                break;
            case SQLITE_NULL:
                row.insert_or_assign(col_name, Value::null());
                break;
            default:
                THROW_UNREACHABLE;
            }
        }

        result_rows.push_back(Value::create(std::move(row)));
    }

    return Value::create(std::move(result_rows));
}

int Connection::exec(const String& sql)
{
    require_open();

    char* errmsg = nullptr;
    int rc = sqlite3_exec(conn_, sql.c_str(), nullptr, nullptr, &errmsg);
    if (rc != SQLITE_OK)
    {
        std::string msg = errmsg;
        sqlite3_free(errmsg);
        throw Frost_Recoverable_Error{msg};
    }
    return sqlite3_changes(conn_);
}

} // namespace frst::sqlite
