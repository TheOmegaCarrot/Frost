#include <frost/ast.hpp>
#include <frost/backtrace.hpp>
#include <frost/builtin.hpp>
#include <frost/ext.hpp>
#include <frost/import.hpp>
#include <frost/meta.hpp>
#include <frost/parser.hpp>
#include <frost/prelude.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <fmt/format.h>
#include <lyra/lyra.hpp>

#include <iostream>
#include <optional>
#include <ranges>
#include <string_view>

using namespace std::literals;

namespace
{

std::string format_backtrace(const frst::Backtrace& bt)
{
    std::string out;

    for (const auto& frame : bt.frames() | std::views::reverse)
    {
        std::visit(
            frst::Overload{
                [&](const frst::Resolved_AST_Frame& f) {
                    fmt::format_to(std::back_inserter(out), "{} [{}]\n",
                                   f.node_label, f.source_range);
                },
                [&](const frst::Call_Frame& f) {
                    fmt::format_to(std::back_inserter(out), "Call ({})\n",
                                   f.function_name);
                },
                [&](const frst::Import_Frame& f) {
                    fmt::format_to(std::back_inserter(out),
                                   "Import Boundary ({})\n", f.module_spec);
                },
                [&](const frst::Iterative_Frame& f) {
                    fmt::format_to(std::back_inserter(out), "{} ({})\n",
                                   f.operation, f.function_name);
                },
            },
            frame);
    }

    return out;
}

} // namespace

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
    catch (frst::Frost_Error& err)
    {
        if (auto bt = err.pilfer_backtrace())
            fmt::print(stderr, "{}\nTraceback:\n{}", err.what(),
                       format_backtrace(*bt));
        else
            fmt::println(stderr, "{}", err.what());
        std::exit(1);
    }
}

void repl(frst::Symbol_Table& symbols, frst::Backtrace_State* bt);
int main(int argc, const char** argv)
{
    const std::span args{argv, argv + argc};
    std::vector<std::string> strings_to_evaluate;
    std::optional<std::filesystem::path> file_to_evaluate;
    std::vector<std::string> args_for_frost;
    bool skip_prelude = false;
    bool do_repl = false;
    bool do_dump = false;
    bool show_help = false;
    bool show_version = false;
    bool do_backtrace = false;

    if (argc == 1)
        do_repl = true;

    std::vector<std::string> pre_args;
    std::vector<std::string> post_args;
    bool after_double_dash = false;

    for (const auto& arg : args | std::views::drop(1))
    {
        if (!after_double_dash && arg == "--"sv)
        {
            after_double_dash = true;
            continue;
        }

        if (after_double_dash)
            post_args.emplace_back(arg);
        else
            pre_args.emplace_back(arg);
    }

    std::string file_arg;
    std::vector<std::string> extra_args;

    auto cli = lyra::cli()
               | lyra::help(show_help)
               | lyra::opt(show_version)["--version"]("Show version and exit.")
               | lyra::opt(do_dump)["-d"]["--dump"](
                   "Dump the AST instead of executing.")
               | lyra::opt(do_repl)["-i"]["--interactive"]("Start the REPL.")
               | lyra::opt(skip_prelude)["--no-prelude"]("Skip prelude")
               | lyra::opt(do_backtrace)["--enable-backtrace"](
                   "Enable backtrace on error")
               | lyra::opt(strings_to_evaluate, "code")["-e"]["--eval"](
                   "Evaluate a snippet of Frost code.")
               | lyra::arg(file_arg, "file")("File to execute.").optional()
               | lyra::arg(extra_args,
                           "args")("Arguments passed to the Frost program.");

    std::vector<const char*> parse_argv;
    parse_argv.reserve(pre_args.size() + 1);
    parse_argv.push_back(args.front());
    parse_argv.append_range(
        std::views::transform(pre_args, &std::string::c_str));

    auto result =
        cli.parse({static_cast<int>(parse_argv.size()), parse_argv.data()});
    if (!result)
    {
        fmt::println(stderr, "{}", result.message());
        return 1;
    }

    if (show_help)
    {
        std::cerr << cli;
        return 0;
    }

    if (show_version)
    {
        fmt::println("frost {}", FROST_VERSION);
        return 0;
    }

    if (!file_arg.empty())
    {
        file_to_evaluate.emplace(file_arg);
        args_for_frost.emplace_back(std::move(file_arg));
    }

    args_for_frost.append_range(extra_args);
    args_for_frost.append_range(post_args);

    if (do_dump && do_repl)
    {
        std::fputs("Refusing to dump repl\n", stderr);
        return 1;
    }

    frst::Backtrace_State trace;
    frst::Backtrace_State* bt = do_backtrace ? &trace : nullptr;

    frst::Symbol_Table symbols;
    frst::inject_builtins(symbols, bt);
    frst::inject_ext(symbols);
    frst::inject_meta(symbols);

    frst::Execution_Context setup_ctx{.symbols = symbols,
                                      .runtime = {.backtrace = bt}};

    if (not skip_prelude)
        frst::inject_prelude(setup_ctx);

    std::vector<std::filesystem::path> module_search_path;
    if (file_to_evaluate)
        module_search_path.push_back(file_to_evaluate.value().parent_path());
    module_search_path.push_back(".");

    module_search_path.append_range(frst::env_module_path());

    frst::inject_import(symbols, module_search_path, bt);

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

    if (do_repl)
        repl(symbols, bt);
}
