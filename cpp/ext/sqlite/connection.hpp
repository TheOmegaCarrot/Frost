#ifndef FROST_EXT_SQLITE_CONNECTION_HPP
#define FROST_EXT_SQLITE_CONNECTION_HPP

#include <frost/builtins-common.hpp>

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
                                              int openmode)
    {
        auto conn = std::make_shared<Connection>(Restricted{});

        int rc =
            sqlite3_open_v2(filename.c_str(), &conn->conn_, openmode, nullptr);
        if (rc != SQLITE_OK)
        {
            std::string msg = sqlite3_errmsg(conn->conn_);
            conn->close();
            throw Frost_Recoverable_Error{msg};
        }

        return conn;
    }

    int exec(const String& sql)
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

    void close()
    {
        require_open();

        sqlite3_close_v2(conn_);
        conn_ = nullptr;
    }

    ~Connection()
    {
        if (conn_)
            sqlite3_close_v2(conn_);
    }

  private:
    void require_open()
    {
        if (not conn_)
            throw Frost_Recoverable_Error{"Database connection is closed"};
    }

    sqlite3* conn_ = nullptr;
};
} // namespace frst::sqlite

#endif
