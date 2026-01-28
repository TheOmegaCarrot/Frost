#include "builtins-common.hpp"

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <fstream>
#include <iostream>
#include <sstream>

namespace frst
{

struct Keys
{
#define KEY(X) static inline const auto X = #X##_s
    KEY(close);
    KEY(is_open);
    KEY(read_line);
    KEY(read_one);
    KEY(read_rest);
    KEY(read);
    KEY(tell);
    KEY(seek);
    KEY(eof);
    KEY(write);
    KEY(get);
#undef KEY
} static const keys;

namespace
{

template <typename T>
auto system_closure(std::size_t min_args, std::size_t max_args, T&& fn)
{
    return Value::create(Function{std::make_shared<Builtin>(
        std::forward<T>(fn), "<system closure>",
        Builtin::Arity{.min = min_args, .max = max_args})});
}

template <typename fstream_t>
auto close(const std::shared_ptr<fstream_t>& stream)
{
    return system_closure(0, 0, [=](builtin_args_t) {
        stream->close();
        return Value::null();
    });
}

template <typename fstream_t>
auto is_open(const std::shared_ptr<fstream_t>& stream)
{
    return system_closure(0, 0, [=](builtin_args_t) {
        return Value::create(stream->is_open());
    });
}

auto eof(const std::shared_ptr<std::istream>& stream)
{
    return system_closure(0, 0, [=](builtin_args_t) {
        return Value::create(stream->eof());
    });
}

auto read_line(const std::shared_ptr<std::istream>& stream)
{
    return system_closure(0, 0, [=](builtin_args_t) {
        std::string line;
        std::getline(*stream, line);
        return Value::create(std::move(line));
    });
}

auto read_one(const std::shared_ptr<std::istream>& stream)
{
    return system_closure(0, 0, [=](builtin_args_t) {
        int got = stream->get();
        if (std::char_traits<char>::not_eof(got))
            return Value::create(String{static_cast<char>(got)});
        else
            return Value::null();
    });
}

auto read_rest(const std::shared_ptr<std::istream>& stream)
{
    return system_closure(0, 0, [=](builtin_args_t) {
        return Value::create(String(std::istreambuf_iterator<char>(*stream),
                                    std::istreambuf_iterator<char>{}));
    });
}

auto tell(const std::shared_ptr<std::istream>& stream)
{
    return system_closure(0, 0, [=](builtin_args_t) {
        return Value::create(Int{stream->tellg()});
    });
}

auto seek(const std::shared_ptr<std::istream>& stream)
{
    return system_closure(0, 1, [=](builtin_args_t args) {
        REQUIRE_ARGS("<system closure: seek>", TYPES(Int));
        stream->seekg(GET(0, Int));
        return Value::null();
    });
}

auto tell(const std::shared_ptr<std::ostream>& stream)
{
    return system_closure(0, 0, [=](builtin_args_t) {
        return Value::create(Int{stream->tellp()});
    });
}

auto seek(const std::shared_ptr<std::ostream>& stream)
{
    return system_closure(0, 1, [=](builtin_args_t args) {
        REQUIRE_ARGS("<system closure: seek>", TYPES(Int));
        stream->seekp(GET(0, Int));
        return Value::null();
    });
}

auto write(const std::shared_ptr<std::ostream>& stream)
{
    return system_closure(0, 1, [=](builtin_args_t args) {
        REQUIRE_ARGS("<system_closure: write>", TYPES(String));
        const auto& str = GET(0, String);
        stream->write(str.c_str(), str.length());
        return Value::null();
    });
}

} // namespace

BUILTIN(open_read)
{
    REQUIRE_ARGS("open_read", TYPES(String));

    auto stream = std::make_shared<std::ifstream>(GET(0, String));

    if (not stream->is_open())
        return Value::null();

    return Value::create(Map{
        {keys.read_line, read_line(stream)},
        {keys.read_one, read_one(stream)},
        {keys.read_rest, read_rest(stream)},
        {keys.close, close(stream)},
        {keys.is_open, is_open(stream)},
        {keys.eof, eof(stream)},
        {keys.tell, tell(stream)},
        {keys.seek, seek(stream)},
    });
}

BUILTIN(open_write)
{
    REQUIRE_ARGS("open_write", TYPES(String));

    auto stream = std::make_shared<std::ofstream>(GET(0, String));

    if (not stream->is_open())
        return Value::null();

    return Value::create(Map{
        {keys.write, write(stream)},
        {keys.tell, tell(stream)},
        {keys.seek, seek(stream)},
        {keys.close, close(stream)},
        {keys.is_open, is_open(stream)},
    });
}

BUILTIN(stringreader)
{
    REQUIRE_ARGS("stringreader", TYPES(String));

    auto stream = std::make_shared<std::istringstream>(GET(0, String));

    return Value::create(Map{
        {keys.read_line, read_line(stream)},
        {keys.read_one, read_one(stream)},
        {keys.read_rest, read_rest(stream)},
        {keys.eof, eof(stream)},
        {keys.tell, tell(stream)},
        {keys.seek, seek(stream)},
    });
}

BUILTIN(stringwriter)
{
    auto stream = std::make_shared<std::ostringstream>();

    return Value::create(
        Map{{keys.write, write(stream)},
            {keys.tell, tell(stream)},
            {keys.seek, seek(stream)},
            {keys.get, system_closure(0, 0, [=](builtin_args_t) {
                 return Value::create(stream->str());
             })}});
}

Value_Ptr make_stdin()
{
    // A shared ptr with a no-op delete...
    // I'm not proud of this...
    std::shared_ptr<std::istream> hacky_stream_ptr(&std::cin, [](auto&&...) {
    });
    return Value::create(Map{
        {keys.read_line, read_line(hacky_stream_ptr)},
        {keys.read_one, read_one(hacky_stream_ptr)},
        {keys.read, read_rest(hacky_stream_ptr)},
    });
}

void inject_streams(Symbol_Table& table)
{
    INJECT(open_read, 1, 1);
    INJECT(open_write, 1, 1);
    INJECT(stringreader, 1, 1);
    INJECT(stringwriter, 0, 0);

    table.define("stdin", make_stdin());
}
} // namespace frst
