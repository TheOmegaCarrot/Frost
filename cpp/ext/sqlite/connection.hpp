#ifndef FROST_EXT_SQLITE_CONNECTION_HPP
#define FROST_EXT_SQLITE_CONNECTION_HPP

#include <frost/builtins-common.hpp>

#include <mutex>
#include <sqlite3.h>

namespace frst::sqlite
{
// Connection wraps a SQLite database handle with RAII lifetime.
//
// All methods except in_transaction() require an open connection and will
// throw Frost_Recoverable_Error if called after close().
// in_transaction() returns false on a closed connection.
class Connection : public std::enable_shared_from_this<Connection>
{
    class Restricted
    {
        friend Connection;
        Restricted() = default;
    };

  public:
    Connection(Restricted)
    {
    }

    Connection() = delete;
    Connection(const Connection&) = delete;
    Connection(Connection&&) = delete;

    static std::shared_ptr<Connection> create(const String& filename,
                                              int openmode);

    int exec(const String& sql, const Array& bindings);
    int exec(const String& sql, const Map& bindings);

    int script(const String& sql);

    void for_each_row(const String& sql, const Array& bindings,
                      std::function<void(Value_Ptr)> row_fn);
    void for_each_row(const String& sql, const Map& bindings,
                      std::function<void(Value_Ptr)> row_fn);

    void close();

    bool in_transaction() const;
    int total_changes();
    Int last_insert_rowid();

  private:
    void require_open_();

    using Stmt_Ptr =
        std::unique_ptr<sqlite3_stmt, decltype([](sqlite3_stmt* s) {
                            sqlite3_finalize(s);
                        })>;

    int exec_impl_(const Stmt_Ptr& stmt);
    void for_each_row_impl_(const Stmt_Ptr& stmt,
                            std::function<void(Value_Ptr)> row_fn);

    Stmt_Ptr prepare_(const String& sql);
    void bind_positional_(const Stmt_Ptr& stmt, const Array& bindings);
    void bind_named_(const Stmt_Ptr& stmt, const Map& bindings);
    static void bind_value_(const Stmt_Ptr& stmt, int pos,
                            const Value_Ptr& val_ptr,
                            const std::string& location);

    Value_Ptr read_row_(const Stmt_Ptr& stmt, int num_cols);

    using Conn_Ptr = std::unique_ptr<sqlite3, decltype([](sqlite3* db) {
                                         sqlite3_close_v2(db);
                                     })>;

    Conn_Ptr conn_;
    std::recursive_mutex mutex_;
};
} // namespace frst::sqlite

#endif
