#include <frost/builtins-common.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <fstream>
#include <iostream>
#include <sstream>

namespace frst
{

STRINGS(close, is_open, read_line, read_one, read_rest, read, tell, seek, eof,
        write, writeln, get);

namespace
{

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
    return system_closure(1, 1, [=](builtin_args_t args) {
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
    return system_closure(1, 1, [=](builtin_args_t args) {
        REQUIRE_ARGS("<system closure: seek>", TYPES(Int));
        stream->seekp(GET(0, Int));
        return Value::null();
    });
}

auto write(const std::shared_ptr<std::ostream>& stream)
{
    return system_closure(1, 1, [=](builtin_args_t args) {
        REQUIRE_ARGS("<system_closure: write>", TYPES(String));
        const auto& str = GET(0, String);
        stream->write(str.c_str(), str.length());
        return Value::null();
    });
}

auto writeln(const std::shared_ptr<std::ostream>& stream)
{
    return system_closure(1, 1, [=](builtin_args_t args) {
        REQUIRE_ARGS("<system_closure: writeln>", TYPES(String));
        const auto& str = GET(0, String);
        stream->write(str.c_str(), str.length());
        stream->put('\n');
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
        {strings.read_line, read_line(stream)},
        {strings.read_one, read_one(stream)},
        {strings.read_rest, read_rest(stream)},
        {strings.close, close(stream)},
        {strings.is_open, is_open(stream)},
        {strings.eof, eof(stream)},
        {strings.tell, tell(stream)},
        {strings.seek, seek(stream)},
    });
}

BUILTIN(open_trunc)
{
    REQUIRE_ARGS("open_trunc", TYPES(String));

    auto stream =
        std::make_shared<std::ofstream>(GET(0, String), std::ios::trunc);

    if (not stream->is_open())
        return Value::null();

    return Value::create(Map{
        {strings.write, write(stream)},
        {strings.writeln, writeln(stream)},
        {strings.tell, tell(stream)},
        {strings.seek, seek(stream)},
        {strings.close, close(stream)},
        {strings.is_open, is_open(stream)},
    });
}

BUILTIN(open_append)
{
    REQUIRE_ARGS("open_append", TYPES(String));

    auto stream =
        std::make_shared<std::ofstream>(GET(0, String), std::ios::app);

    if (not stream->is_open())
        return Value::null();

    return Value::create(Map{
        {strings.write, write(stream)},
        {strings.writeln, writeln(stream)},
        {strings.tell, tell(stream)},
        {strings.seek, seek(stream)},
        {strings.close, close(stream)},
        {strings.is_open, is_open(stream)},
    });
}

BUILTIN(stringreader)
{
    REQUIRE_ARGS("stringreader", TYPES(String));

    auto stream = std::make_shared<std::istringstream>(GET(0, String));

    return Value::create(Map{
        {strings.read_line, read_line(stream)},
        {strings.read_one, read_one(stream)},
        {strings.read_rest, read_rest(stream)},
        {strings.eof, eof(stream)},
        {strings.tell, tell(stream)},
        {strings.seek, seek(stream)},
    });
}

BUILTIN(stringwriter)
{
    auto stream = std::make_shared<std::ostringstream>();

    return Value::create(
        Map{{strings.write, write(stream)},
            {strings.writeln, writeln(stream)},
            {strings.tell, tell(stream)},
            {strings.seek, seek(stream)},
            {strings.get, system_closure(0, 0, [=](builtin_args_t) {
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
        {strings.read_line, read_line(hacky_stream_ptr)},
        {strings.read_one, read_one(hacky_stream_ptr)},
        {strings.read, read_rest(hacky_stream_ptr)},
    });
}

void inject_streams(Symbol_Table& table)
{
    INJECT(open_read, 1, 1);
    INJECT(open_trunc, 1, 1);
    INJECT(open_append, 1, 1);
    INJECT(stringreader, 1, 1);
    INJECT(stringwriter, 0, 0);

    table.define("stdin", make_stdin());
}
} // namespace frst
