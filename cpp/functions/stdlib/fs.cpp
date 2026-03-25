#include <frost/builtins-common.hpp>

#include <frost/value.hpp>

#include <boost/hof/lift.hpp>

#include <filesystem>

using namespace std::literals;
namespace stdf = std::filesystem;

namespace frst
{

namespace fs
{

namespace
{

void throw_if_error(std::error_code ec)
{
    if (ec)
        throw Frost_Recoverable_Error{ec.message()};
}

// Template helpers

template <auto Function, typename R = void>
Value_Ptr fs_unary(std::string_view name, builtin_args_t args)
{
    REQUIRE_ARGS(name, PARAM("path", TYPES(String)));
    std::error_code ec;
    if constexpr (std::is_void_v<R>)
    {
        Function(GET(0, String), ec);
        throw_if_error(ec);
        return Value::null();
    }
    else
    {
        R res = Function(GET(0, String), ec);
        throw_if_error(ec);
        return Value::create(std::move(res));
    }
}

template <auto Function>
Value_Ptr fs_binary_void(std::string_view name, std::string_view p1,
                         std::string_view p2, builtin_args_t args)
{
    REQUIRE_ARGS(name, PARAM(p1, TYPES(String)), PARAM(p2, TYPES(String)));
    std::error_code ec;
    Function(GET(0, String), GET(1, String), ec);
    throw_if_error(ec);
    return Value::null();
}

template <auto Member>
Value_Ptr fs_path_component(std::string_view name, builtin_args_t args)
{
    REQUIRE_ARGS(name, PARAM("path", TYPES(String)));
    stdf::path input = GET(0, String);
    return Value::create(String{(input.*Member)()});
}

} // namespace

BUILTIN(move)
{
    return fs_binary_void<BOOST_HOF_LIFT(stdf::rename)>("fs.move", "src",
                                                        "dest", args);
}

BUILTIN(symlink)
{
    return fs_binary_void<BOOST_HOF_LIFT(stdf::create_symlink)>(
        "fs.symlink", "to", "link", args);
}

BUILTIN(copy)
{
    REQUIRE_ARGS("fs.copy", PARAM("src", TYPES(String)),
                 PARAM("dest", TYPES(String)));

    std::error_code ec;
    stdf::copy(GET(0, String), GET(1, String), stdf::copy_options::recursive,
               ec);
    throw_if_error(ec);
    return Value::null();
}

BUILTIN(absolute)
{
    return fs_unary<BOOST_HOF_LIFT(stdf::absolute), String>("fs.absolute",
                                                            args);
}
BUILTIN(canonical)
{
    return fs_unary<BOOST_HOF_LIFT(stdf::canonical), String>("fs.canonical",
                                                             args);
}
BUILTIN(exists)
{
    return fs_unary<BOOST_HOF_LIFT(stdf::exists), Bool>("fs.exists", args);
}
BUILTIN(remove)
{
    return fs_unary<BOOST_HOF_LIFT(stdf::remove), Bool>("fs.remove", args);
}
BUILTIN(remove_recursively)
{
    return fs_unary<BOOST_HOF_LIFT(stdf::remove_all), Int>(
        "fs.remove_recursively", args);
}
BUILTIN(mkdir)
{
    return fs_unary<BOOST_HOF_LIFT(stdf::create_directories), Bool>("fs.mkdir",
                                                                    args);
}
BUILTIN(size)
{
    return fs_unary<BOOST_HOF_LIFT(stdf::file_size), Int>("fs.size", args);
}

BUILTIN(cd)
{
    return fs_unary<BOOST_HOF_LIFT(stdf::current_path)>("fs.cd", args);
}

BUILTIN(cwd)
{
    std::error_code ec;
    String res = stdf::current_path(ec);
    throw_if_error(ec);
    return Value::create(std::move(res));
}

BUILTIN(parent)
{
    return fs_path_component<&stdf::path::parent_path>("fs.parent", args);
}
BUILTIN(stem)
{
    return fs_path_component<&stdf::path::stem>("fs.stem", args);
}
BUILTIN(filename)
{
    return fs_path_component<&stdf::path::filename>("fs.filename", args);
}
BUILTIN(extension)
{
    return fs_path_component<&stdf::path::extension>("fs.extension", args);
}

namespace
{
template <typename directory_iterator_t>
Value_Ptr list_impl(const String& path)
{

    std::error_code ec;
    auto itr = directory_iterator_t(
        path, stdf::directory_options::skip_permission_denied, ec);
    throw_if_error(ec);
    auto end = directory_iterator_t{};

    Array result;
    for (; itr != end; itr.increment(ec))
    {
        if (ec)
        {
            ec.clear();
            continue;
        }
        result.push_back(Value::create(auto{itr->path().string()}));
    }
    return Value::create(std::move(result));
}
} // namespace

BUILTIN(list)
{
    REQUIRE_ARGS("fs.list", PARAM("path", TYPES(String)));

    return list_impl<stdf::directory_iterator>(GET(0, String));
}

BUILTIN(list_recursively)
{
    REQUIRE_ARGS("fs.list_recursively", PARAM("path", TYPES(String)));

    return list_impl<stdf::recursive_directory_iterator>(GET(0, String));
}

BUILTIN(stat)
{
    REQUIRE_ARGS("fs.stat", PARAM("path", TYPES(String)));

    STRINGS(type, none, not_found, regular, directory, symlink, block,
            character, fifo, socket, unknown, perms, owner, group, others, read,
            write, exec);

    auto type_to_str = [](stdf::file_type type) {
        switch (type)
        {
            using enum stdf::file_type;
#define X_TYPES                                                                \
    X(none)                                                                    \
    X(not_found)                                                               \
    X(regular)                                                                 \
    X(directory)                                                               \
    X(symlink)                                                                 \
    X(block)                                                                   \
    X(character)                                                               \
    X(fifo)                                                                    \
    X(socket)                                                                  \
    X(unknown)
#define X(TYPE)                                                                \
    case TYPE:                                                                 \
        return strings.TYPE;
            X_TYPES
#undef X
        default:
            THROW_UNREACHABLE;
        }
    };

    std::error_code ec;
    auto status = stdf::status(GET(0, String), ec);
    throw_if_error(ec);

    const auto& perms = status.permissions();
    using enum stdf::perms;

    auto has = [&](stdf::perms p) {
        return Value::create(Bool{(perms & p) != stdf::perms::none});
    };

    Map result{
        {strings.type, type_to_str(status.type())},
        {
            strings.perms,
            Value::create(
                Value::trusted,
                Map{
                    {strings.owner,
                     Value::create(Value::trusted,
                                   Map{
                                       {strings.read, has(owner_read)},
                                       {strings.write, has(owner_write)},
                                       {strings.exec, has(owner_exec)},
                                   })},
                    {strings.group,
                     Value::create(Value::trusted,
                                   Map{
                                       {strings.read, has(group_read)},
                                       {strings.write, has(group_write)},
                                       {strings.exec, has(group_exec)},
                                   })},
                    {strings.others,
                     Value::create(Value::trusted,
                                   Map{
                                       {strings.read, has(others_read)},
                                       {strings.write, has(others_write)},
                                       {strings.exec, has(others_exec)},
                                   })},
                }),
        },
    };

    return Value::create(Value::trusted, std::move(result));
}

BUILTIN(concat)
{
    REQUIRE_ARGS("fs.concat", PARAM("base", TYPES(String)),
                 PARAM("path", TYPES(String)));

    return Value::create(String{
        (stdf::path{GET(0, String)} / stdf::path{GET(1, String)}).string()});
}

} // namespace fs

STDLIB_MODULE(fs, ENTRY(move, 2), ENTRY(symlink, 2), ENTRY(copy, 2),
              ENTRY(absolute, 1), ENTRY(canonical, 1), ENTRY(cd, 1),
              ENTRY(cwd, 0), ENTRY(exists, 1), ENTRY(remove, 1),
              ENTRY(remove_recursively, 1), ENTRY(mkdir, 1), ENTRY(size, 1),
              ENTRY(stat, 1), ENTRY(list, 1), ENTRY(list_recursively, 1),
              ENTRY(concat, 2), ENTRY(stem, 1), ENTRY(parent, 1),
              ENTRY(filename, 1), ENTRY(extension, 1))

} // namespace frst
