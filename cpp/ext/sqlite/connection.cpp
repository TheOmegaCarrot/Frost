#include "connection.hpp"

namespace frst::sqlite
{

std::shared_ptr<Connection> Connection::create(const String& filename,
                                               int openmode)
{
    auto conn = std::make_shared<Connection>(Restricted{});

    int rc = sqlite3_open_v2(filename.c_str(), std::out_ptr(conn->conn_),
                             openmode, nullptr);
    if (rc != SQLITE_OK)
    {
        std::string msg = sqlite3_errmsg(conn->conn_.get());
        throw Frost_Recoverable_Error{msg};
    }

    return conn;
}

void Connection::close()
{
    std::lock_guard lock{mutex_};
    require_open_();

    conn_.reset();
}

int Connection::script(const String& sql)
{
    std::lock_guard lock{mutex_};
    require_open_();

    int before = sqlite3_total_changes(conn_.get());

    char* errmsg = nullptr;
    int rc = sqlite3_exec(conn_.get(), sql.c_str(), nullptr, nullptr, &errmsg);
    if (rc != SQLITE_OK)
    {
        std::string msg = errmsg ? errmsg : "unknown error";
        sqlite3_free(errmsg);
        throw Frost_Recoverable_Error{msg};
    }

    return sqlite3_total_changes(conn_.get()) - before;
}

int Connection::exec_impl_(const Stmt_Ptr& stmt)
{
    int before = sqlite3_total_changes(conn_.get());

    while (true)
    {
        int rc = sqlite3_step(stmt.get());
        if (rc == SQLITE_DONE)
            break;
        if (rc != SQLITE_ROW)
        {
            std::string msg = sqlite3_errmsg(conn_.get());
            throw Frost_Recoverable_Error{msg};
        }
    }

    return sqlite3_total_changes(conn_.get()) - before;
}

int Connection::exec(const String& sql, const Array& bindings)
{
    std::lock_guard lock{mutex_};
    auto stmt = prepare_(sql);
    bind_positional_(stmt, bindings);
    return exec_impl_(stmt);
}

int Connection::exec(const String& sql, const Map& bindings)
{
    std::lock_guard lock{mutex_};
    auto stmt = prepare_(sql);
    bind_named_(stmt, bindings);
    return exec_impl_(stmt);
}

void Connection::for_each_row_impl_(const Stmt_Ptr& stmt,
                                    const std::function<void(Value_Ptr)>& row_fn)
{
    int num_cols = sqlite3_column_count(stmt.get());
    while (true)
    {
        int rc = sqlite3_step(stmt.get());
        if (rc == SQLITE_DONE)
            break;
        if (rc != SQLITE_ROW)
        {
            std::string msg = sqlite3_errmsg(conn_.get());
            throw Frost_Recoverable_Error{msg};
        }

        row_fn(read_row_(stmt, num_cols));
    }
}

void Connection::for_each_row(const String& sql, const Array& bindings,
                              std::function<void(Value_Ptr)> row_fn)
{
    std::lock_guard lock{mutex_};
    auto stmt = prepare_(sql);
    bind_positional_(stmt, bindings);
    for_each_row_impl_(stmt, std::move(row_fn));
}

void Connection::for_each_row(const String& sql, const Map& bindings,
                              std::function<void(Value_Ptr)> row_fn)
{
    std::lock_guard lock{mutex_};
    auto stmt = prepare_(sql);
    bind_named_(stmt, bindings);
    for_each_row_impl_(stmt, std::move(row_fn));
}

bool Connection::in_transaction() const
{
    return conn_ && not sqlite3_get_autocommit(conn_.get());
}

int Connection::total_changes()
{
    require_open_();
    return sqlite3_total_changes(conn_.get());
}

Int Connection::last_insert_rowid()
{
    require_open_();
    return sqlite3_last_insert_rowid(conn_.get());
}

void Connection::require_open_()
{
    if (not conn_)
        throw Frost_Recoverable_Error{"Database connection is closed"};
}

Connection::Stmt_Ptr Connection::prepare_(const String& sql)
{
    require_open_();

    Stmt_Ptr stmt;
    const char* tail = nullptr;

    int rc = sqlite3_prepare_v2(conn_.get(), sql.c_str(),
                                static_cast<int>(sql.size()),
                                std::out_ptr(stmt), &tail);
    if (rc != SQLITE_OK)
    {
        std::string msg = sqlite3_errmsg(conn_.get());
        throw Frost_Recoverable_Error{msg};
    }

    if (not stmt)
        throw Frost_Recoverable_Error{"empty SQL statement"};

    if (tail)
    {
        std::string_view remaining{tail};
        auto pos = remaining.find_first_not_of(" \t\n\r;");
        if (pos != std::string_view::npos)
        {
            throw Frost_Recoverable_Error{
                "query contains trailing content after the first statement"};
        }
    }

    return stmt;
}

void Connection::bind_value_(const Stmt_Ptr& stmt, int pos,
                             const Value_Ptr& val_ptr,
                             const std::string& location)
{
    auto* s = stmt.get();
    val_ptr->visit(Overload{
        [&](const Null&) {
            sqlite3_bind_null(s, pos);
        },
        [&](const Int& v) {
            sqlite3_bind_int64(s, pos, v);
        },
        [&](const Float& v) {
            sqlite3_bind_double(s, pos, v);
        },
        [&](const Bool& v) {
            sqlite3_bind_int64(s, pos, v ? 1 : 0);
        },
        [&](const String& v) {
            sqlite3_bind_text(s, pos, v.c_str(), static_cast<int>(v.size()),
                              SQLITE_TRANSIENT);
        },
        [&](const auto&) {
            throw Frost_Recoverable_Error{"unsupported binding type at "
                                          + location};
        },
    });
}

void Connection::bind_positional_(const Stmt_Ptr& stmt, const Array& bindings)
{
    int param_count = sqlite3_bind_parameter_count(stmt.get());
    if (param_count != static_cast<int>(bindings.size()))
    {
        throw Frost_Recoverable_Error{
            fmt::format("query expected {} binding{}, got {}", param_count,
                        param_count == 1 ? "" : "s", bindings.size())};
    }

    for (const auto& [idx, val_ptr] : std::views::enumerate(bindings))
    {
        bind_value_(stmt, static_cast<int>(idx) + 1, val_ptr,
                    "index " + std::to_string(idx));
    }
}

void Connection::bind_named_(const Stmt_Ptr& stmt, const Map& bindings)
{
    int param_count = sqlite3_bind_parameter_count(stmt.get());

    // Validate that all parameters are named
    for (int i = 1; i <= param_count; ++i)
    {
        if (not sqlite3_bind_parameter_name(stmt.get(), i))
            throw Frost_Recoverable_Error{
                "cannot use named bindings with positional (?) parameters"};
    }

    if (param_count != static_cast<int>(bindings.size()))
    {
        throw Frost_Recoverable_Error{
            fmt::format("query expected {} binding{}, got {}", param_count,
                        param_count == 1 ? "" : "s", bindings.size())};
    }

    for (const auto& [key, val_ptr] : bindings)
    {
        if (not key->is<String>())
            throw Frost_Recoverable_Error{
                fmt::format("named binding keys must be strings, got {}",
                            key->type_name())};
        auto& key_str = key->raw_get<String>();

        // Try :name, @name, $name
        int pos =
            sqlite3_bind_parameter_index(stmt.get(), (":" + key_str).c_str());
        if (not pos)
            pos = sqlite3_bind_parameter_index(stmt.get(),
                                               ("@" + key_str).c_str());
        if (not pos)
            pos = sqlite3_bind_parameter_index(stmt.get(),
                                               ("$" + key_str).c_str());
        if (not pos)
            throw Frost_Recoverable_Error{
                "no parameter named '" + key_str + "' in query"};

        bind_value_(stmt, pos, val_ptr, "'" + key_str + "'");
    }
}

Value_Ptr Connection::read_row_(const Stmt_Ptr& stmt, int num_cols)
{
    auto* s = stmt.get();
    Map row;
    for (int i = 0; i != num_cols; ++i)
    {
        auto col_name = Value::create(String{sqlite3_column_name(s, i)});
        switch (sqlite3_column_type(s, i))
        {
        case SQLITE_INTEGER:
            row.insert_or_assign(
                col_name, Value::create(Int{sqlite3_column_int64(s, i)}));
            break;
        case SQLITE_FLOAT:
            row.insert_or_assign(col_name,
                                 Value::create(sqlite3_column_double(s, i)));
            break;
        case SQLITE_TEXT:
            row.insert_or_assign(
                col_name,
                Value::create(String{
                    reinterpret_cast<const char*>(sqlite3_column_text(s, i)),
                    static_cast<std::size_t>(sqlite3_column_bytes(s, i)),
                }));
            break;
        case SQLITE_BLOB:
            row.insert_or_assign(
                col_name,
                Value::create(String{
                    static_cast<const char*>(sqlite3_column_blob(s, i)),
                    static_cast<std::size_t>(sqlite3_column_bytes(s, i)),
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
