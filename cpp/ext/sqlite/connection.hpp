#ifndef FROST_EXT_SQLITE_CONNECTION_HPP
#define FROST_EXT_SQLITE_CONNECTION_HPP

#include <frost/builtins-common.hpp>

#include <mutex>
#include <sqlite3.h>

namespace frst::sqlite
{
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

    int script(const String& sql);

    void for_each_row(const String& sql, const Array& bindings,
                      std::function<void(Value_Ptr)> row_fn);

    void close();

    ~Connection();

  private:
    void require_open_();

    sqlite3_stmt* prepare_and_bind_(const String& sql, const Array& bindings);

    Value_Ptr read_row_(sqlite3_stmt* stmt, int num_cols);

    sqlite3* conn_ = nullptr;
    std::recursive_mutex mutex_;
};
} // namespace frst::sqlite

#endif
