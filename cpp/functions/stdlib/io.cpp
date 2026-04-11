#include <frost/builtins-common.hpp>
#include <frost/streams.hpp>

#include <frost/stdlib.hpp>
#include <frost/value.hpp>

#include <fstream>
#include <sstream>

namespace frst
{

using namespace streams_detail;

STRINGS(close, is_open, read_line, read_one, read_rest, tell, seek, eof, write,
        writeln, get, flush);

namespace
{
[[noreturn]] void open_error(const String& filename)
{
    throw Frost_Recoverable_Error{
        fmt::format("Failed to open file: {}", filename)};
}
} // namespace

namespace io
{

BUILTIN(open_read)
{
    REQUIRE_ARGS("io.open_read", PARAM("path", TYPES(String)));

    const auto& filename = GET(0, String);

    auto ls = std::make_shared<Locked_Stream<std::ifstream>>();
    ls->stream = std::make_shared<std::ifstream>(filename);

    if (not ls->stream->is_open())
        open_error(filename);

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
    REQUIRE_ARGS("io.open_trunc", PARAM("path", TYPES(String)));

    const auto& filename = GET(0, String);

    auto ls = std::make_shared<Locked_Stream<std::ofstream>>();
    ls->stream = std::make_shared<std::ofstream>(filename, std::ios::trunc);

    if (not ls->stream->is_open())
        open_error(filename);

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
    REQUIRE_ARGS("io.open_append", PARAM("path", TYPES(String)));

    const auto& filename = GET(0, String);

    auto ls = std::make_shared<Locked_Stream<std::ofstream>>();
    ls->stream = std::make_shared<std::ofstream>(filename, std::ios::app);

    if (not ls->stream->is_open())
        open_error(filename);

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
    REQUIRE_ARGS("io.stringreader", TYPES(String));

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
    REQUIRE_NULLARY("io.stringwriter");

    auto ls = std::make_shared<Locked_Stream<std::ostringstream>>();
    ls->stream = std::make_shared<std::ostringstream>();

    return Value::create(
        Value::trusted,
        Map{{strings.write, write(ls)},
            {strings.writeln, writeln(ls)},
            {strings.tell, tell(ls)},
            {strings.seek, seek(ls)},
            {strings.get, system_closure([ls](builtin_args_t args) {
                 REQUIRE_NULLARY("io.stringwriter.get");
                 std::lock_guard lock{ls->mutex};
                 return Value::create(ls->stream->str());
             })}});
}

} // namespace io

STDLIB_MODULE(io, ENTRY(open_read), ENTRY(open_trunc), ENTRY(open_append),
              ENTRY(stringreader), ENTRY(stringwriter))

} // namespace frst
