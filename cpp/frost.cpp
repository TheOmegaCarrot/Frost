#include <frost/ast.hpp>
#include <frost/backtrace.hpp>
#include <frost/builtin.hpp>
#include <frost/ext.hpp>
#include <frost/import.hpp>
#include <frost/meta.hpp>
#include <frost/parser.hpp>
#include <frost/prelude.hpp>
#include <frost/stdlib.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <fmt/format.h>

#include <iostream>
#include <optional>
#include <ranges>
#include <string_view>

using namespace std::literals;

void exec_program(const std::vector<frst::ast::Statement::Ptr>& program,
                  frst::Execution_Context ctx, bool do_dump)
{
    try
    {
        for (const auto& statement : program)
        {
            if (do_dump)
                statement->debug_dump_ast(std::cout);
            else
                statement->execute(ctx);
        }
    }
    catch (frst::Frost_User_Error& e)
    {
        auto bt = e.take_backtrace();
        if (!bt.empty())
            fmt::println(stderr, "Error: {}\nTraceback:\n{}", e.what(),
                         fmt::join(bt, "\n"));
        else
            fmt::println(stderr, "Error: {}", e.what());

        std::exit(1);
    }
    catch (frst::Frost_Interpreter_Error& e)
    {
        auto bt = e.take_backtrace();
        if (!bt.empty())
            fmt::println(stderr, "INTERNAL ERROR: {}\nTraceback:\n{}", e.what(),
                         fmt::join(bt, "\n"));
        else
            fmt::println(stderr, "INTERNAL ERROR: {}", e.what());

        std::exit(1);
    }
}

constexpr std::string_view HELP_TEXT =
    R"(Usage: frost [options] [file [args...]]

Run a Frost script, or start the REPL with no arguments.

Options:
  -h, --help             Show this help message and exit
      --version          Show version and exit
  -d, --dump             Dump the AST instead of executing
  -i, --interactive      Start the REPL after any -e or file
      --no-prelude       Skip loading the prelude
      --enable-backtrace Enable backtrace on error
  -e, --eval <code>      Evaluate a snippet of Frost code (repeatable)

Driver options end at the first non-flag argument (the script file)
or an explicit `--` terminator. Everything after passes through to
the script verbatim as `args`.
)";

void repl(frst::Symbol_Table& symbols);
int main(int argc, const char** argv)
{
    const std::span argv_span{argv, argv + argc};
    std::vector<std::string> strings_to_evaluate;
    std::optional<std::filesystem::path> file_to_evaluate;
    std::vector<std::string> args_for_frost;
    bool skip_prelude = false;
    bool do_repl = false;
    bool do_dump = false;
    bool do_backtrace = false;

    // Parse driver flags in a single pass.
    //
    // Driver flags are everything up to:
    //   - the first non-flag argument (the script file), OR
    //   - an explicit `--` terminator.
    //
    // Everything after that point (including the script file itself)
    // passes through to the script verbatim -- so a script can freely
    // use any flag name without the driver stealing it.
    std::size_t i = 1;
    while (i < argv_span.size())
    {
        std::string_view arg = argv_span[i];

        if (arg == "--")
        {
            ++i; // swallow the terminator
            break;
        }

        if (not arg.starts_with('-') || arg == "-")
            break; // first non-flag: start of script args

        auto take_value = [&](std::string_view name) -> std::string_view {
            if (i + 1 >= argv_span.size())
            {
                fmt::println(stderr, "frost: option '{}' requires a value",
                             name);
                std::exit(1);
            }
            return argv_span[++i];
        };

        if (arg == "-h" || arg == "--help" || arg == "-?")
        {
            fmt::print("{}", HELP_TEXT);
            return 0;
        }
        else if (arg == "--version")
        {
            fmt::println("frost {}", FROST_VERSION);
            return 0;
        }
        else if (arg == "-d" || arg == "--dump")
            do_dump = true;
        else if (arg == "-i" || arg == "--interactive")
            do_repl = true;
        else if (arg == "--no-prelude")
            skip_prelude = true;
        else if (arg == "--enable-backtrace")
            do_backtrace = true;
        else if (arg == "-e" || arg == "--eval")
            strings_to_evaluate.emplace_back(take_value(arg));
        else
        {
            fmt::println(stderr, "frost: unknown option '{}'", arg);
            return 1;
        }

        ++i;
    }

    // Everything else is script args, verbatim.
    std::vector<std::string> script_args(std::from_range,
                                         argv_span | std::views::drop(i));

    // When `-e` is used, the code provided via -e is the entry point and
    // script_args are pure pass-through arguments. Otherwise, the first
    // script arg (if any) is the file to run.
    if (strings_to_evaluate.empty() && not script_args.empty())
        file_to_evaluate.emplace(script_args.front());

    args_for_frost = std::move(script_args);

    if (do_dump && do_repl)
    {
        std::fputs("Refusing to dump repl\n", stderr);
        return 1;
    }

    frst::Backtrace_State trace;
    frst::Backtrace_State::set_current(do_backtrace ? &trace : nullptr);

    frst::Symbol_Table symbols;
    frst::inject_builtins(symbols);
    frst::inject_meta(symbols);

    frst::Execution_Context setup_ctx{.symbols = symbols};

    if (not skip_prelude)
        frst::inject_prelude(setup_ctx);

    std::vector<std::filesystem::path> module_search_path;
    if (file_to_evaluate)
        module_search_path.push_back(file_to_evaluate.value().parent_path());
    module_search_path.push_back(".");

    module_search_path.append_range(frst::env_module_path());

    frst::Stdlib_Registry_Builder builder;
    frst::register_stdlib(builder);
    frst::register_extensions(builder);
    frst::inject_import(
        symbols, module_search_path,
        std::make_shared<frst::Stdlib_Registry>(std::move(builder).build()));

    symbols.define("args", frst::Value::create(
                               args_for_frost
                               | std::views::transform([](auto arg) {
                                     return frst::Value::create(std::move(arg));
                                 })
                               | std::ranges::to<frst::Array>()));

    symbols.define("imported", frst::Value::create(false));

    for (const auto& cli_program : strings_to_evaluate)
    {
        auto results = frst::parse_program(cli_program);
        if (not results)
        {
            fmt::println(stderr, "{}", results.error());
            return 1;
        }
        exec_program(results.value(), setup_ctx, do_dump);
    }

    if (file_to_evaluate)
    {
        auto results = frst::parse_file(file_to_evaluate.value());
        if (not results)
        {
            fmt::println(stderr, "{}", results.error());
            return 1;
        }
        exec_program(results.value(), setup_ctx, do_dump);
    }

    if (not file_to_evaluate && strings_to_evaluate.empty())
        do_repl = true;

    if (do_repl)
        repl(symbols);
}
