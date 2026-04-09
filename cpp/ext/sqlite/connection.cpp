#include "connection.hpp"

#include <extensions.h>

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

    auto* db = conn->conn_.get();
    sqlite3_decimal_init(db, nullptr, nullptr);
    sqlite3_csv_init(db, nullptr, nullptr);
    sqlite3_fileio_init(db, nullptr, nullptr);
    sqlite3_regexp_init(db, nullptr, nullptr);
    sqlite3_series_init(db, nullptr, nullptr);
    sqlite3_shathree_init(db, nullptr, nullptr);
    sqlite3_sqlar_init(db, nullptr, nullptr);
    sqlite3_uuid_init(db, nullptr, nullptr);
    sqlite3_zipfile_init(db, nullptr, nullptr);

    return conn;
}

void Connection::close()
{
    std::lock_guard lock{mutex_};
    require_open_();

    conn_.reset();
}

namespace
{

Value_Ptr value_to_frost(sqlite3_value* val)
{
    switch (sqlite3_value_type(val))
    {
    case SQLITE_INTEGER:
        return Value::create(Int{sqlite3_value_int64(val)});
    case SQLITE_FLOAT:
        return Value::create(sqlite3_value_double(val));
    case SQLITE_TEXT:
        return Value::create(
            String{reinterpret_cast<const char*>(sqlite3_value_text(val)),
                   static_cast<std::size_t>(sqlite3_value_bytes(val))});
    case SQLITE_BLOB:
        return Value::create(
            String{static_cast<const char*>(sqlite3_value_blob(val)),
                   static_cast<std::size_t>(sqlite3_value_bytes(val))});
    case SQLITE_NULL:
    default:
        return Value::null();
    }
}

void result_to_sqlite(sqlite3_context* ctx, const Value_Ptr& val)
{
    val->visit(Overload{
        [&](const Null&) {
            sqlite3_result_null(ctx);
        },
        [&](const Int& v) {
            sqlite3_result_int64(ctx, v);
        },
        [&](const Float& v) {
            sqlite3_result_double(ctx, v);
        },
        [&](const Bool& v) {
            sqlite3_result_int64(ctx, v ? 1 : 0);
        },
        [&](const String& v) {
            sqlite3_result_text(ctx, v.c_str(), static_cast<int>(v.size()),
                                SQLITE_TRANSIENT);
        },
        [&](const auto&) {
            sqlite3_result_error(ctx, "unsupported return type", -1);
        },
    });
}

void scalar_callback(sqlite3_context* ctx, int argc, sqlite3_value** argv)
{
    auto* fn = static_cast<Function*>(sqlite3_user_data(ctx));

    Array args;
    args.reserve(static_cast<std::size_t>(argc));
    for (int i = 0; i < argc; ++i)
        args.push_back(value_to_frost(argv[i]));

    try
    {
        auto result = (*fn)->call(args);
        result_to_sqlite(ctx, result);
    }
    catch (const Frost_Error& e)
    {
        sqlite3_result_error(ctx, e.what(), -1);
    }
}

void function_destroy(void* user_data)
{
    delete static_cast<Function*>(user_data);
}

int trace_callback(unsigned mask, void* user_data, void* stmt_ptr, void*)
{
    if (mask != SQLITE_TRACE_STMT)
        return 0;

    auto* fn = static_cast<Function*>(user_data);
    auto* stmt = static_cast<sqlite3_stmt*>(stmt_ptr);

    char* expanded = sqlite3_expanded_sql(stmt);
    String sql = expanded ? String{expanded} : String{sqlite3_sql(stmt)};
    sqlite3_free(expanded);

    try
    {
        (*fn)->call({Value::create(std::move(sql))});
    }
    catch (const Frost_Recoverable_Error& e)
    {
        fmt::println(stderr, "error in trace callback: {}", e.what());
    }
    catch (const Frost_Unrecoverable_Error& e)
    {
        fmt::println(stderr, "fatal error in trace callback: {}", e.what());
        std::exit(1);
    }
    catch (const Frost_Interpreter_Error& e)
    {
        fmt::println(stderr, "internal error in trace callback: {}", e.what());
        std::exit(1);
    }

    return 0;
}

struct Aggregate_User_Data
{
    Value_Ptr init;
    Function step_fn;
    std::optional<Function> finalize_fn;
};

struct Aggregate_Accumulator
{
    Value_Ptr acc;
    std::optional<std::string> step_error;
};

void aggregate_step(sqlite3_context* ctx, int argc, sqlite3_value** argv)
{
    auto* ud = static_cast<Aggregate_User_Data*>(sqlite3_user_data(ctx));
    auto** agg_pp = static_cast<Aggregate_Accumulator**>(
        sqlite3_aggregate_context(ctx, sizeof(Aggregate_Accumulator*)));
    if (not agg_pp)
        return;

    if (not *agg_pp)
        *agg_pp = new Aggregate_Accumulator{ud->init, std::nullopt};

    auto* agg = *agg_pp;

    if (agg->step_error)
        return;

    Array args;
    args.reserve(static_cast<std::size_t>(argc) + 1);
    args.push_back(agg->acc);
    for (int i = 0; i < argc; ++i)
        args.push_back(value_to_frost(argv[i]));

    try
    {
        agg->acc = ud->step_fn->call(args);
    }
    catch (const Frost_Error& e)
    {
        agg->step_error = e.what();
    }
}

void aggregate_final(sqlite3_context* ctx)
{
    auto* ud = static_cast<Aggregate_User_Data*>(sqlite3_user_data(ctx));
    auto** agg_pp = static_cast<Aggregate_Accumulator**>(
        sqlite3_aggregate_context(ctx, sizeof(Aggregate_Accumulator*)));

    std::unique_ptr<Aggregate_Accumulator> agg;
    if (agg_pp and *agg_pp)
    {
        agg.reset(*agg_pp);
        *agg_pp = nullptr;
    }

    Value_Ptr final_acc = agg ? agg->acc : ud->init;

    if (agg and agg->step_error)
    {
        sqlite3_result_error(ctx, agg->step_error.value().c_str(), -1);
        return;
    }

    try
    {
        if (ud->finalize_fn)
        {
            auto result = ud->finalize_fn.value()->call({final_acc});
            result_to_sqlite(ctx, result);
        }
        else
        {
            result_to_sqlite(ctx, final_acc);
        }
    }
    catch (const Frost_Error& e)
    {
        sqlite3_result_error(ctx, e.what(), -1);
    }
}

void aggregate_destroy(void* user_data)
{
    delete static_cast<Aggregate_User_Data*>(user_data);
}

} // namespace

void Connection::create_function(const String& name, const Function& fn)
{
    std::lock_guard lock{mutex_};
    require_open_();

    auto* fn_copy = new Function{fn};

    int rc = sqlite3_create_function_v2(conn_.get(), name.c_str(), -1,
                                        SQLITE_UTF8, fn_copy, scalar_callback,
                                        nullptr, nullptr, function_destroy);
    if (rc != SQLITE_OK)
    {
        delete fn_copy;
        throw Frost_Recoverable_Error{
            fmt::format("Failed to create SQL function '{}': {}", name,
                        sqlite3_errmsg(conn_.get()))};
    }
}

void Connection::create_aggregate(const String& name, const Value_Ptr& init,
                                  const Function& step,
                                  std::optional<Function> finalize)
{
    std::lock_guard lock{mutex_};
    require_open_();

    auto* ud = new Aggregate_User_Data{init, step, std::move(finalize)};

    int rc = sqlite3_create_function_v2(
        conn_.get(), name.c_str(), -1, SQLITE_UTF8, ud, nullptr, aggregate_step,
        aggregate_final, aggregate_destroy);
    if (rc != SQLITE_OK)
    {
        delete ud;
        throw Frost_Recoverable_Error{
            fmt::format("Failed to create SQL aggregate '{}': {}", name,
                        sqlite3_errmsg(conn_.get()))};
    }
}

void Connection::trace(std::optional<Function> fn)
{
    std::lock_guard lock{mutex_};
    require_open_();

    if (fn)
    {
        auto* fn_copy = new Function{std::move(fn.value())};
        sqlite3_trace_v2(conn_.get(), SQLITE_TRACE_STMT, trace_callback,
                         fn_copy);
        trace_fn_.reset(fn_copy);
    }
    else
    {
        sqlite3_trace_v2(conn_.get(), 0, nullptr, nullptr);
        trace_fn_.reset();
    }
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

void Connection::for_each_row_impl_(
    const Stmt_Ptr& stmt, const std::function<void(Value_Ptr)>& row_fn)
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
    std::lock_guard lock{mutex_};
    require_open_();
    return sqlite3_total_changes(conn_.get());
}

Int Connection::last_insert_rowid()
{
    std::lock_guard lock{mutex_};
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
    int rc = val_ptr->visit(Overload{
        [&](const Null&) {
            return sqlite3_bind_null(s, pos);
        },
        [&](const Int& v) {
            return sqlite3_bind_int64(s, pos, v);
        },
        [&](const Float& v) {
            return sqlite3_bind_double(s, pos, v);
        },
        [&](const Bool& v) {
            return sqlite3_bind_int64(s, pos, v ? 1 : 0);
        },
        [&](const String& v) {
            return sqlite3_bind_text(s, pos, v.c_str(),
                                     static_cast<int>(v.size()),
                                     SQLITE_TRANSIENT);
        },
        [&](const auto&) -> int {
            throw Frost_Recoverable_Error{"unsupported binding type at "
                                          + location};
        },
    });

    if (rc != SQLITE_OK)
        throw Frost_Recoverable_Error{fmt::format(
            "failed to bind value at {}: {}", location, sqlite3_errstr(rc))};
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
