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

int Connection::script(const String& sql)
{
    require_open_();

    int before = sqlite3_total_changes(conn_);

    char* errmsg = nullptr;
    int rc = sqlite3_exec(conn_, sql.c_str(), nullptr, nullptr, &errmsg);
    if (rc != SQLITE_OK)
    {
        std::string msg = errmsg ? errmsg : "unknown error";
        sqlite3_free(errmsg);
        throw Frost_Recoverable_Error{msg};
    }

    return sqlite3_total_changes(conn_) - before;
}

int Connection::exec(const String& sql, const Array& bindings)
{
    sqlite3_stmt* stmt = prepare_and_bind_(sql, bindings);
    BOOST_SCOPE_EXIT_ALL(&)
    {
        sqlite3_finalize(stmt);
    };

    int before = sqlite3_total_changes(conn_);

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
    }

    return sqlite3_total_changes(conn_) - before;
}

void Connection::for_each_row(const String& sql, const Array& bindings,
                              std::function<void(Value_Ptr)> row_fn)
{
    sqlite3_stmt* stmt = prepare_and_bind_(sql, bindings);
    BOOST_SCOPE_EXIT_ALL(&)
    {
        sqlite3_finalize(stmt);
    };

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

        row_fn(read_row_(stmt, num_cols));
    }
}

void Connection::require_open_()
{
    if (not conn_)
        throw Frost_Recoverable_Error{"Database connection is closed"};
}

sqlite3_stmt* Connection::prepare_and_bind_(const String& sql,
                                            const Array& bindings)
{
    require_open_();

    sqlite3_stmt* stmt = nullptr;
    const char* tail = nullptr;

    int rc = sqlite3_prepare_v2(conn_, sql.c_str(),
                                static_cast<int>(sql.size()), &stmt, &tail);
    if (rc != SQLITE_OK)
    {
        std::string msg = sqlite3_errmsg(conn_);
        sqlite3_finalize(stmt);
        throw Frost_Recoverable_Error{msg};
    }

    // Empty or whitespace-only SQL produces a null statement
    if (not stmt)
        throw Frost_Recoverable_Error{"empty SQL statement"};

    // Reject trailing content after the single statement
    if (tail)
    {
        std::string_view remaining{tail};
        auto pos = remaining.find_first_not_of(" \t\n\r;");
        if (pos != std::string_view::npos)
        {
            sqlite3_finalize(stmt);
            throw Frost_Recoverable_Error{
                "query contains trailing content after the first statement"};
        }
    }

    // Validate binding count
    int param_count = sqlite3_bind_parameter_count(stmt);
    if (param_count != static_cast<int>(bindings.size()))
    {
        sqlite3_finalize(stmt);
        throw Frost_Recoverable_Error{
            fmt::format("query expected {} binding{}, got {}", param_count,
                        param_count == 1 ? "" : "s", bindings.size())};
    }

    // Bind parameters
    for (const auto& [idx, val_ptr] : std::views::enumerate(bindings))
    {
        int bind_pos = static_cast<int>(idx) + 1;
        val_ptr->visit(Overload{
            [&](const Null&) {
                sqlite3_bind_null(stmt, bind_pos);
            },
            [&](const Int& v) {
                sqlite3_bind_int64(stmt, bind_pos, v);
            },
            [&](const Float& v) {
                sqlite3_bind_double(stmt, bind_pos, v);
            },
            [&](const Bool& v) {
                sqlite3_bind_int64(stmt, bind_pos, v ? 1 : 0);
            },
            [&](const String& v) {
                sqlite3_bind_text(stmt, bind_pos, v.c_str(),
                                  static_cast<int>(v.size()), SQLITE_TRANSIENT);
            },
            [&](const auto&) {
                sqlite3_finalize(stmt);
                throw Frost_Recoverable_Error{
                    "unsupported binding type at index " + std::to_string(idx)};
            },
        });
    }

    return stmt;
}

Value_Ptr Connection::read_row_(sqlite3_stmt* stmt, int num_cols)
{
    Map row;
    for (int i = 0; i != num_cols; ++i)
    {
        auto col_name = Value::create(String{sqlite3_column_name(stmt, i)});
        switch (sqlite3_column_type(stmt, i))
        {
        case SQLITE_INTEGER:
            row.insert_or_assign(
                col_name, Value::create(Int{sqlite3_column_int64(stmt, i)}));
            break;
        case SQLITE_FLOAT:
            row.insert_or_assign(col_name,
                                 Value::create(sqlite3_column_double(stmt, i)));
            break;
        case SQLITE_TEXT:
            row.insert_or_assign(
                col_name,
                Value::create(String{
                    reinterpret_cast<const char*>(sqlite3_column_text(stmt, i)),
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
    return Value::create(std::move(row));
}

} // namespace frst::sqlite
