#include "connection.hpp"

namespace frst::sqlite
{

std::shared_ptr<Connection> Connection::create(const String& filename,
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
