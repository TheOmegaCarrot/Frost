#include <frost/builtins-common.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <filesystem>

using namespace std::literals;

namespace frst
{

namespace fs
{

namespace
{
void throw_if_error(std::error_code ec)
{
    if (ec != std::error_code{})
        throw Frost_Recoverable_Error{ec.message()};
}
} // namespace

#define BINARY_FS_VOID(frost_name, cpp_name, p1name, p2name)                   \
    BUILTIN(frost_name)                                                        \
    {                                                                          \
        REQUIRE_ARGS("fs." #frost_name, PARAM(#p1name, TYPES(String)),         \
                     PARAM(#p2name, TYPES(String)));                           \
                                                                               \
        std::error_code ec;                                                    \
        std::filesystem::cpp_name(GET(0, String), GET(1, String), ec);         \
        throw_if_error(ec);                                                    \
        return Value::null();                                                  \
    }

BINARY_FS_VOID(move, rename, src, dest)
BINARY_FS_VOID(symlink, create_symlink, to, link)

BUILTIN(copy)
{
    REQUIRE_ARGS("fs.copy", PARAM("src", TYPES(String)),
                 PARAM("dest", TYPES(String)));

    std::error_code ec;
    using enum std::filesystem::copy_options;
    std::filesystem::copy(GET(0, String), GET(1, String), recursive, ec);
    throw_if_error(ec);
    return Value::null();
}

#define UNARY_FS(frost_name, cpp_name, R)                                      \
    BUILTIN(frost_name)                                                        \
    {                                                                          \
        REQUIRE_ARGS("fs." #frost_name, TYPES(String));                        \
                                                                               \
        std::error_code ec;                                                    \
        R res = std::filesystem::cpp_name(GET(0, String), ec);                 \
        throw_if_error(ec);                                                    \
        return Value::create(std::move(res));                                  \
    }

UNARY_FS(absolute, absolute, String)
UNARY_FS(canonical, canonical, String)
UNARY_FS(exists, exists, Bool)
UNARY_FS(remove, remove, Bool)
UNARY_FS(remove_recursively, remove_all, Int)
UNARY_FS(mkdir, create_directories, Bool)
UNARY_FS(size, file_size, Int)

#define UNARY_FS_VOID(frost_name, cpp_name)                                    \
    BUILTIN(frost_name)                                                        \
    {                                                                          \
        REQUIRE_ARGS("fs." #frost_name, TYPES(String));                        \
                                                                               \
        std::error_code ec;                                                    \
        std::filesystem::cpp_name(GET(0, String), ec);                         \
        throw_if_error(ec);                                                    \
        return Value::null();                                                  \
    }

UNARY_FS_VOID(cd, current_path)

#define NULLARY_FS(frost_name, cpp_name)                                       \
    BUILTIN(frost_name)                                                        \
    {                                                                          \
        std::error_code ec;                                                    \
        String res = std::filesystem::cpp_name(ec);                            \
        throw_if_error(ec);                                                    \
        return Value::create(std::move(res));                                  \
    }

NULLARY_FS(cwd, current_path)

BUILTIN(list)
{
    REQUIRE_ARGS("fs.list", TYPES(String));

    std::error_code ec;
    auto itr = std::filesystem::directory_iterator(GET(0, String), ec);
    throw_if_error(ec);
    auto end = std::filesystem::directory_iterator{};

    Array result;
    for (; itr != end; ++itr)
    {
        result.push_back(Value::create(auto{itr->path().native()}));
    }
    return Value::create(std::move(result));
}

BUILTIN(list_recursively)
{
    REQUIRE_ARGS("fs.list", TYPES(String));

    std::error_code ec;
    auto itr =
        std::filesystem::recursive_directory_iterator(GET(0, String), ec);
    throw_if_error(ec);
    auto end = std::filesystem::recursive_directory_iterator{};

    Array result;
    for (; itr != end; ++itr)
    {
        result.push_back(Value::create(auto{itr->path().native()}));
    }
    return Value::create(std::move(result));
}

BUILTIN(stat)
{
    REQUIRE_ARGS("fs.perms", TYPES(String));

    STRINGS(type, none, not_found, regular, directory, symlink, block,
            character, fifo, socket, unknown, perms, owner, group, others, read,
            write, exec);

    auto type_to_str = [](std::filesystem::file_type type) {
        switch (type)
        {
            using enum std::filesystem::file_type;
#define X_TYPES                                                                \
    X(none)                                                                    \
    X(not_found)                                                               \
    X(regular)                                                                 \
    X(directory) X(symlink) X(block) X(character) X(fifo) X(socket) X(unknown)
#define X(type)                                                                \
    case type:                                                                 \
        return strings.type;
            X_TYPES
#undef X
        default:
            THROW_UNREACHABLE;
        }
    };

    std::error_code ec;
    auto status = std::filesystem::status(GET(0, String), ec);
    throw_if_error(ec);

    const auto& perms = status.permissions();
    using enum std::filesystem::perms;
    Map result{{strings.type, type_to_str(status.type())},
               {strings.perms,
                Value::create(Map{
                    {strings.owner,
                     Value::create(Map{
                         {strings.read,
                          Value::create(
                              Bool{(perms & owner_read)
                                   != static_cast<std::filesystem::perms>(0)})},
                         {strings.write,
                          Value::create(
                              Bool{(perms & owner_write)
                                   != static_cast<std::filesystem::perms>(0)})},
                         {strings.exec,
                          Value::create(
                              Bool{(perms & owner_exec)
                                   != static_cast<std::filesystem::perms>(0)})},
                     })},
                    {strings.group,
                     Value::create(Map{
                         {strings.read,
                          Value::create(
                              Bool{(perms & group_read)
                                   != static_cast<std::filesystem::perms>(0)})},
                         {strings.write,
                          Value::create(
                              Bool{(perms & group_write)
                                   != static_cast<std::filesystem::perms>(0)})},
                         {strings.exec,
                          Value::create(
                              Bool{(perms & group_exec)
                                   != static_cast<std::filesystem::perms>(0)})},
                     })},
                    {strings.others,
                     Value::create(Map{
                         {strings.read,
                          Value::create(
                              Bool{(perms & others_read)
                                   != static_cast<std::filesystem::perms>(0)})},
                         {strings.write,
                          Value::create(
                              Bool{(perms & others_write)
                                   != static_cast<std::filesystem::perms>(0)})},
                         {strings.exec,
                          Value::create(
                              Bool{(perms & others_exec)
                                   != static_cast<std::filesystem::perms>(0)})},
                     })}})}};

    return Value::create(std::move(result));
}

BUILTIN(concat)
{
    REQUIRE_ARGS("concat", TYPES(String), TYPES(String));

    return Value::create(String{(std::filesystem::path{GET(0, String)}
                                 / std::filesystem::path{GET(1, String)})
                                    .native()});
}

} // namespace fs

void inject_filesystem(Symbol_Table& table)
{
    using namespace fs;
    INJECT_MAP(fs, ENTRY(move, 2, 2), ENTRY(symlink, 2, 2), ENTRY(copy, 2, 2),
               ENTRY(absolute, 1, 1), ENTRY(canonical, 1, 1), ENTRY(cd, 1, 1),
               ENTRY(cwd, 0, 0), ENTRY(exists, 1, 1), ENTRY(remove, 1, 1),
               ENTRY(remove_recursively, 1, 1), ENTRY(mkdir, 1, 1),
               ENTRY(size, 1, 1), ENTRY(stat, 1, 1), ENTRY(list, 1, 1),
               ENTRY(list_recursively, 1, 1), ENTRY(concat, 2, 2));
}
} // namespace frst
