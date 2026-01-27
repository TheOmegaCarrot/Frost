#include <frost/ast.hpp>
#include <frost/builtin.hpp>
#include <frost/parser.hpp>
#include <frost/prelude.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <fmt/format.h>
#include <fmt/ranges.h>

#include <algorithm>
#include <iostream>
#include <optional>
#include <ranges>
#include <string_view>

using namespace std::literals;

void exec_program(const std::vector<frst::ast::Statement::Ptr>& program,
                  frst::Symbol_Table& symbols, bool do_dump)
{
    try
    {
        for (const auto& statement : program)
        {
            if (do_dump)
                statement->debug_dump_ast(std::cout);
            else
                statement->execute(symbols);
        }
    }
    catch (const frst::Frost_Error& err)
    {
        fmt::println(stderr, "{}", err.what());
        std::exit(1);
    }
}

void repl(frst::Symbol_Table& symbols);
int main(int argc, const char** argv)
{
    const std::span args{argv, argv + argc};
    std::vector<std::string> strings_to_evaluate;
    std::optional<std::filesystem::path> file_to_evaluate;
    std::vector<std::string> args_for_frost;
    bool do_repl = false;
    bool do_dump = false;

    if (argc == 1)
        do_repl = true;

    // The Frost driver's arguments are so simple I'll just handle them by hand

    bool collect_rest = false;
    bool skip = false;
    for (const auto& [current, next] :
         std::views::concat(args, std::views::single(""))
             | std::views::drop(1)
             | std::views::adjacent<2>)
    {
        if (skip)
        {
            skip = false;
            continue;
        }

        if (collect_rest)
        {
            args_for_frost.emplace_back(current);
            continue;
        }

        if (current == "--"sv)
        {
            collect_rest = true;
            continue;
        }

        if (current == "-e"sv)
        {
            skip = true;
            strings_to_evaluate.emplace_back(next);
            continue;
        }

        if (current == "-d"sv)
        {
            do_dump = true;
            continue;
        }

        if (current == "-i"sv)
        {
            do_repl = true;
            continue;
        }

        if (not file_to_evaluate)
        {
            file_to_evaluate.emplace(current);
            args_for_frost.emplace_back(current);
            continue;
        }

        if (current != ""sv)
            args_for_frost.emplace_back(current);
    }

    if (do_dump && do_repl)
    {
        std::fputs("Refusing to dump repl", stderr);
        return 1;
    }

    // fmt::println(R"(strings to eval: {}
    // file to eval: {}
    // args for frost: {}
    // )",
    //              strings_to_evaluate,
    //              file_to_evaluate.value_or("{none}").native(),
    //              args_for_frost);

    frst::Symbol_Table symbols;
    frst::inject_builtins(symbols);
    frst::inject_prelude(symbols);

    symbols.define("args", frst::Value::create(
                               args_for_frost
                               | std::views::transform([](auto arg) {
                                     return frst::Value::create(std::move(arg));
                                 })
                               | std::ranges::to<frst::Array>()));

    for (const auto& cli_program : strings_to_evaluate)
    {
        auto results = frst::parse_program(cli_program);
        if (not results)
        {
            fmt::println(stderr, "{}", results.error());
            return 1;
        }
        exec_program(results.value(), symbols, do_dump);
    }

    if (file_to_evaluate)
    {
        auto results = frst::parse_file(file_to_evaluate.value());
        if (not results)
        {
            fmt::println(stderr, "{}", results.error());
            return 1;
        }
        exec_program(results.value(), symbols, do_dump);
    }

    if (do_repl)
        repl(symbols);
}
