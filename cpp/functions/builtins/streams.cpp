#include <frost/builtins-common.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <fstream>
#include <iostream>
#include <mutex>
#include <sstream>

namespace frst
{

STRINGS(close, is_open, read_line, read_one, read_rest, read, tell, seek, eof,
        write, writeln, get, flush);

namespace
{

template <typename Stream>
struct Locked_Stream
{
    std::shared_ptr<Stream> stream;
    std::mutex mutex;
};

template <typename fstream_t>
auto close(const std::shared_ptr<Locked_Stream<fstream_t>>& ls)
{
    return system_closure(0, 0, [ls](builtin_args_t) {
        std::lock_guard lock{ls->mutex};
        ls->stream->close();
        return Value::null();
    });
}

template <typename fstream_t>
auto is_open(const std::shared_ptr<Locked_Stream<fstream_t>>& ls)
{
    return system_closure(0, 0, [ls](builtin_args_t) {
        std::lock_guard lock{ls->mutex};
        return Value::create(ls->stream->is_open());
    });
}

template <std::derived_from<std::istream> Stream>
auto eof(const std::shared_ptr<Locked_Stream<Stream>>& ls)
{
    return system_closure(0, 0, [ls](builtin_args_t) {
        std::lock_guard lock{ls->mutex};
        return Value::create(ls->stream->eof());
    });
}

template <std::derived_from<std::istream> Stream>
auto read_line(const std::shared_ptr<Locked_Stream<Stream>>& ls)
{
    return system_closure(0, 0, [ls](builtin_args_t) {
        std::lock_guard lock{ls->mutex};
        std::string line;
        std::getline(*ls->stream, line);
        return Value::create(std::move(line));
    });
}

template <std::derived_from<std::istream> Stream>
auto read_one(const std::shared_ptr<Locked_Stream<Stream>>& ls)
{
    return system_closure(0, 0, [ls](builtin_args_t) {
        std::lock_guard lock{ls->mutex};
        int got = ls->stream->get();
        if (std::char_traits<char>::not_eof(got))
            return Value::create(String{static_cast<char>(got)});
        else
            return Value::null();
    });
}

template <std::derived_from<std::istream> Stream>
auto read_rest(const std::shared_ptr<Locked_Stream<Stream>>& ls)
{
    return system_closure(0, 0, [ls](builtin_args_t) {
        std::lock_guard lock{ls->mutex};
        return Value::create(String(std::istreambuf_iterator<char>(*ls->stream),
                                    std::istreambuf_iterator<char>{}));
    });
}

template <std::derived_from<std::istream> Stream>
auto tell(const std::shared_ptr<Locked_Stream<Stream>>& ls)
{
    return system_closure(0, 0, [ls](builtin_args_t) {
        std::lock_guard lock{ls->mutex};
        return Value::create(Int{ls->stream->tellg()});
    });
}

template <std::derived_from<std::istream> Stream>
auto seek(const std::shared_ptr<Locked_Stream<Stream>>& ls)
{
    return system_closure(1, 1, [ls](builtin_args_t args) {
        REQUIRE_ARGS("<system closure:seek>", PARAM("pos", TYPES(Int)));
        std::lock_guard lock{ls->mutex};
        ls->stream->seekg(GET(0, Int));
        return Value::null();
    });
}

template <std::derived_from<std::ostream> Stream>
auto tell(const std::shared_ptr<Locked_Stream<Stream>>& ls)
{
    return system_closure(0, 0, [ls](builtin_args_t) {
        std::lock_guard lock{ls->mutex};
        return Value::create(Int{ls->stream->tellp()});
    });
}

template <std::derived_from<std::ostream> Stream>
auto seek(const std::shared_ptr<Locked_Stream<Stream>>& ls)
{
    return system_closure(1, 1, [ls](builtin_args_t args) {
        REQUIRE_ARGS("<system closure:seek>", PARAM("pos", TYPES(Int)));
        std::lock_guard lock{ls->mutex};
        ls->stream->seekp(GET(0, Int));
        return Value::null();
    });
}

template <std::derived_from<std::ostream> Stream>
auto write(const std::shared_ptr<Locked_Stream<Stream>>& ls)
{
    return system_closure(1, 1, [ls](builtin_args_t args) {
        REQUIRE_ARGS("<system closure:write>", TYPES(String));
        const auto& str = GET(0, String);
        std::lock_guard lock{ls->mutex};
        ls->stream->write(str.c_str(), str.length());
        return Value::null();
    });
}

template <std::derived_from<std::ostream> Stream>
auto writeln(const std::shared_ptr<Locked_Stream<Stream>>& ls)
{
    return system_closure(1, 1, [ls](builtin_args_t args) {
        REQUIRE_ARGS("<system closure:writeln>", TYPES(String));
        const auto& str = GET(0, String);
        std::lock_guard lock{ls->mutex};
        ls->stream->write(str.c_str(), str.length());
        ls->stream->put('\n');
        return Value::null();
    });
}

template <std::derived_from<std::ostream> Stream>
auto flush(const std::shared_ptr<Locked_Stream<Stream>>& ls)
{
    return system_closure(0, 0, [ls](builtin_args_t) {
        std::lock_guard lock{ls->mutex};
        ls->stream->flush();
        return Value::null();
    });
}

} // namespace

BUILTIN(open_read)
{
    REQUIRE_ARGS("open_read", PARAM("path", TYPES(String)));

    auto ls = std::make_shared<Locked_Stream<std::ifstream>>();
    ls->stream = std::make_shared<std::ifstream>(GET(0, String));

    if (not ls->stream->is_open())
        return Value::null();

    return Value::create(Value::trusted, Map{
                                             {strings.read_line, read_line(ls)},
                                             {strings.read_one, read_one(ls)},
                                             {strings.read_rest, read_rest(ls)},
                                             {strings.close, close(ls)},
                                             {strings.is_open, is_open(ls)},
                                             {strings.eof, eof(ls)},
                                             {strings.tell, tell(ls)},
                                             {strings.seek, seek(ls)},
                                         });
}

BUILTIN(open_trunc)
{
    REQUIRE_ARGS("open_trunc", PARAM("path", TYPES(String)));

    auto ls = std::make_shared<Locked_Stream<std::ofstream>>();
    ls->stream =
        std::make_shared<std::ofstream>(GET(0, String), std::ios::trunc);

    if (not ls->stream->is_open())
        return Value::null();

    return Value::create(Value::trusted, Map{
                                             {strings.write, write(ls)},
                                             {strings.writeln, writeln(ls)},
                                             {strings.tell, tell(ls)},
                                             {strings.seek, seek(ls)},
                                             {strings.close, close(ls)},
                                             {strings.is_open, is_open(ls)},
                                             {strings.flush, flush(ls)},
                                         });
}

BUILTIN(open_append)
{
    REQUIRE_ARGS("open_append", PARAM("path", TYPES(String)));

    auto ls = std::make_shared<Locked_Stream<std::ofstream>>();
    ls->stream = std::make_shared<std::ofstream>(GET(0, String), std::ios::app);

    if (not ls->stream->is_open())
        return Value::null();

    return Value::create(Value::trusted, Map{
                                             {strings.write, write(ls)},
                                             {strings.writeln, writeln(ls)},
                                             {strings.tell, tell(ls)},
                                             {strings.seek, seek(ls)},
                                             {strings.close, close(ls)},
                                             {strings.is_open, is_open(ls)},
                                             {strings.flush, flush(ls)},
                                         });
}

BUILTIN(stringreader)
{
    REQUIRE_ARGS("stringreader", TYPES(String));

    auto ls = std::make_shared<Locked_Stream<std::istringstream>>();
    ls->stream = std::make_shared<std::istringstream>(GET(0, String));

    return Value::create(Value::trusted, Map{
                                             {strings.read_line, read_line(ls)},
                                             {strings.read_one, read_one(ls)},
                                             {strings.read_rest, read_rest(ls)},
                                             {strings.eof, eof(ls)},
                                             {strings.tell, tell(ls)},
                                             {strings.seek, seek(ls)},
                                         });
}

BUILTIN(stringwriter)
{
    auto ls = std::make_shared<Locked_Stream<std::ostringstream>>();
    ls->stream = std::make_shared<std::ostringstream>();

    return Value::create(
        Value::trusted,
        Map{{strings.write, write(ls)},
            {strings.writeln, writeln(ls)},
            {strings.tell, tell(ls)},
            {strings.seek, seek(ls)},
            {strings.get, system_closure(0, 0, [ls](builtin_args_t) {
                 std::lock_guard lock{ls->mutex};
                 return Value::create(ls->stream->str());
             })}});
}

Value_Ptr make_stdin()
{
    // A shared ptr to std::cin with a no-op delete...
    // I'm not proud of this...
    auto hacky_stream_ptr = std::make_shared<Locked_Stream<std::istream>>(
        std::shared_ptr<std::istream>(&std::cin, [](auto&&...) {
        }));

    return Value::create(Value::trusted,
                         Map{
                             {strings.read_line, read_line(hacky_stream_ptr)},
                             {strings.read_one, read_one(hacky_stream_ptr)},
                             {strings.read, read_rest(hacky_stream_ptr)},
                         });
}

Value_Ptr make_stderr()
{
    auto ls = std::make_shared<Locked_Stream<std::ostream>>(
        std::shared_ptr<std::ostream>(&std::clog, [](auto&&...) {
        }));

    return Value::create(Value::trusted, Map{
                                             {strings.write, write(ls)},
                                             {strings.writeln, writeln(ls)},
                                         });
}

void inject_streams(Symbol_Table& table)
{
    INJECT(open_read, 1);
    INJECT(open_trunc, 1);
    INJECT(open_append, 1);
    INJECT(stringreader, 1);
    INJECT(stringwriter, 0);

    table.define("stdin", make_stdin());
    table.define("stderr", make_stderr());
}
} // namespace frst
