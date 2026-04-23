#include <frost/builtins-common.hpp>

#include <frost/value.hpp>

#include <boost/version.hpp>
#if BOOST_VERSION >= 108600
#include <boost/process/v1/args.hpp>
#include <boost/process/v1/child.hpp>
#include <boost/process/v1/env.hpp>
#include <boost/process/v1/io.hpp>
#include <boost/process/v1/start_dir.hpp>
namespace bp = boost::process::v1;
#else
#include <boost/process/args.hpp>
#include <boost/process/child.hpp>
#include <boost/process/env.hpp>
#include <boost/process/io.hpp>
#include <boost/process/start_dir.hpp>
namespace bp = boost::process;
#endif

// TODO: Windows portability: replace <unistd.h> with _getpid/_gethostname
// pid and hostname are going to need to be updated for a Windows build once
// MSVC catches up to the C++ features used in Frost
#include <unistd.h>

#include <thread>

namespace frst
{

namespace os
{

BUILTIN(getenv)
{
    REQUIRE_ARGS("os.getenv", TYPES(String));

    const char* val = std::getenv(GET(0, String).c_str());

    if (not val)
        return Value::null();

    return Value::create(String{val});
}

BUILTIN(exit)
{
    REQUIRE_ARGS("os.exit", TYPES(Int));

    std::exit(GET(0, Int));
}

BUILTIN(sleep)
{
    REQUIRE_ARGS("os.sleep", TYPES(Int));

    std::this_thread::sleep_for(std::chrono::milliseconds(GET(0, Int)));

    return Value::null();
}

BUILTIN(pid)
{
    REQUIRE_NULLARY("os.pid");
    return Value::create(Int{getpid()});
}

BUILTIN(hostname)
{
    REQUIRE_NULLARY("os.hostname");

    char buf[256];
    if (gethostname(buf, sizeof(buf)) != 0)
        throw Frost_Recoverable_Error{"os.hostname: failed to get hostname"};

    return Value::create(String{buf});
}

BUILTIN(setenv)
{
    REQUIRE_ARGS("os.setenv", PARAM("variable", TYPES(String)),
                 PARAM("value", TYPES(String)));
    if (::setenv(GET(0, String).c_str(), GET(1, String).c_str(), 1) != 0)
        throw Frost_Recoverable_Error(
            fmt::format("os.setenv: {}", std::strerror(errno)));
    return Value::null();
}

BUILTIN(unsetenv)
{
    REQUIRE_ARGS("os.unsetenv", PARAM("variable", TYPES(String)));

    if (::unsetenv(GET(0, String).c_str()) != 0)
        throw Frost_Recoverable_Error(
            fmt::format("os.unsetenv: {}", std::strerror(errno)));

    return Value::null();
}

namespace
{

struct Run_Options
{
    std::optional<String> stdin_data;
    std::optional<String> cwd;
    std::optional<Map> env;
    std::optional<Map> replace_env;
};

void validate_string_map(const Map& m, std::string_view option_name)
{
    for (const auto& [k, v] : m)
    {
        if (not k->is<String>())
            throw Frost_Recoverable_Error{
                fmt::format("os.run: {} keys must be Strings, got {}",
                            option_name, k->type_name())};
        if (not v->is<String>())
            throw Frost_Recoverable_Error{
                fmt::format("os.run: {} values must be Strings, got {}",
                            option_name, v->type_name())};
    }
}

Run_Options parse_options(const Map& opts)
{
    Run_Options result;

    for (const auto& [k_val, v_val] : opts)
    {
        if (not k_val->is<String>())
            throw Frost_Recoverable_Error{
                fmt::format("os.run: option keys must be Strings, got {}",
                            k_val->type_name())};

        const auto& key = k_val->raw_get<String>();

        if (key == "stdin")
        {
            if (not v_val->is<String>())
                throw Frost_Recoverable_Error{
                    "os.run: stdin option must be a String"};
            result.stdin_data = v_val->raw_get<String>();
        }
        else if (key == "cwd")
        {
            if (not v_val->is<String>())
                throw Frost_Recoverable_Error{
                    "os.run: cwd option must be a String"};
            result.cwd = v_val->raw_get<String>();
        }
        else if (key == "env")
        {
            if (not v_val->is<Map>())
                throw Frost_Recoverable_Error{
                    "os.run: env option must be a Map"};
            validate_string_map(v_val->raw_get<Map>(), "env");
            result.env = v_val->raw_get<Map>();
        }
        else if (key == "replace_env")
        {
            if (not v_val->is<Map>())
                throw Frost_Recoverable_Error{
                    "os.run: replace_env option must be a Map"};
            validate_string_map(v_val->raw_get<Map>(), "replace_env");
            result.replace_env = v_val->raw_get<Map>();
        }
        else
        {
            throw Frost_Recoverable_Error{
                fmt::format("os.run: unknown option '{}'", key)};
        }
    }

    if (result.env && result.replace_env)
        throw Frost_Recoverable_Error{
            "os.run: cannot specify both env and replace_env"};

    return result;
}

std::string read_stream(bp::ipstream& stream)
{
    return {std::istreambuf_iterator<char>(stream), {}};
}

Value_Ptr run_process(const String& exe, std::vector<std::string> cmd_args,
                      const Run_Options& opts)
{
    bp::ipstream stdout_stream;
    bp::ipstream stderr_stream;
    bp::opstream stdin_stream;

    // Build environment
    std::optional<bp::environment> child_env;
    if (opts.replace_env)
    {
        child_env.emplace();
        for (const auto& [k, v] : opts.replace_env.value())
            child_env.value()[k->raw_get<String>()] = v->raw_get<String>();
    }
    else if (opts.env)
    {
        child_env = boost::this_process::environment();
        for (const auto& [k, v] : opts.env.value())
            child_env.value()[k->raw_get<String>()] = v->raw_get<String>();
    }

    // stdin type differs (pipe vs null) so we
    // parameterize over it to avoid duplicating the env/cwd branches.
    auto make_child = [&](auto stdin_init) -> bp::child {
        auto out_init = bp::std_out > stdout_stream;
        auto err_init = bp::std_err > stderr_stream;

        if (child_env && opts.cwd)
            return bp::child(exe, bp::args = cmd_args, out_init, err_init,
                             stdin_init, bp::env = child_env.value(),
                             bp::start_dir = opts.cwd.value());
        if (child_env)
            return bp::child(exe, bp::args = cmd_args, out_init, err_init,
                             stdin_init, bp::env = child_env.value());
        if (opts.cwd)
            return bp::child(exe, bp::args = cmd_args, out_init, err_init,
                             stdin_init, bp::start_dir = opts.cwd.value());
        return bp::child(exe, bp::args = cmd_args, out_init, err_init,
                         stdin_init);
    };

    bp::child proc = [&]() -> bp::child {
        try
        {
            if (opts.stdin_data)
                return make_child(bp::std_in < stdin_stream);
            return make_child(bp::std_in < bp::null);
        }
        catch (const bp::process_error& e)
        {
            throw Frost_Recoverable_Error{fmt::format(
                "os.run: failed to launch '{}': {}", exe, e.what())};
        }
    }();

    // Write stdin, then close to signal EOF
    if (opts.stdin_data)
    {
        stdin_stream << opts.stdin_data.value();
        stdin_stream.flush();
        stdin_stream.pipe().close();
    }

    // Read stdout and stderr concurrently to avoid deadlock when the
    // child fills one pipe buffer while we're blocked reading the other.
    std::string stderr_data;
    std::thread stderr_reader([&] {
        stderr_data = read_stream(stderr_stream);
    });
    auto stdout_data = read_stream(stdout_stream);
    stderr_reader.join();

    proc.wait();

    static const auto key_stdout = "stdout"_s;
    static const auto key_stderr = "stderr"_s;
    static const auto key_exit_code = "exit_code"_s;

    return Value::create(
        Value::trusted,
        Map{{key_stdout, Value::create(String{std::move(stdout_data)})},
            {key_stderr, Value::create(String{std::move(stderr_data)})},
            {key_exit_code, Value::create(Int{proc.exit_code()})}});
}

} // namespace

BUILTIN(run)
{
    REQUIRE_ARGS("os.run", PARAM("command", TYPES(String)),
                 PARAM("args", TYPES(Array)),
                 OPTIONAL(PARAM("options", TYPES(Map))));

    const auto& cmd = GET(0, String);
    const auto& cmd_args = GET(1, Array);

    for (std::size_t i = 0; i < cmd_args.size(); ++i)
        if (not cmd_args[i]->is<String>())
            throw Frost_Recoverable_Error{
                fmt::format("os.run: all args must be Strings, got {} at "
                            "index {}",
                            cmd_args[i]->type_name(), i)};

    auto args_vec = cmd_args
                    | std::views::transform([](const Value_Ptr& arg) {
                          return arg->raw_get<String>();
                      })
                    | std::ranges::to<std::vector>();

    Run_Options opts;
    if (HAS(2))
        opts = parse_options(GET(2, Map));

    return run_process(cmd, std::move(args_vec), opts);
}

} // namespace os

STDLIB_MODULE(os, ENTRY(getenv), ENTRY(setenv), ENTRY(unsetenv), ENTRY(exit),
              ENTRY(sleep), ENTRY(run), ENTRY(pid), ENTRY(hostname))

} // namespace frst
