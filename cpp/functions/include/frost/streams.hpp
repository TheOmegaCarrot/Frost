#ifndef FROST_FUNCTIONS_STREAMS_HPP
#define FROST_FUNCTIONS_STREAMS_HPP

#include <frost/builtins-common.hpp>

#include <memory>
#include <mutex>

namespace frst::streams_detail
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

} // namespace frst::streams_detail

#endif
