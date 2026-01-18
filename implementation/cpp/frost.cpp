#include "frost/value.hpp"
#include <frost/ast.hpp>
#include <frost/builtin.hpp>
#include <frost/parser.hpp>
#include <frost/symbol-table.hpp>

#include <fmt/format.h>
#include <fmt/ranges.h>

#include <replxx.hxx>

#include <iostream>
#include <optional>
#include <ranges>
#include <string_view>

using namespace std::literals;

void repl_exec(const std::vector<frst::ast::Statement::Ptr>& ast,
               frst::Symbol_Table& symbols)
{
    try
    {
        for (const auto& statement : ast
                                         | std::views::reverse
                                         | std::views::drop(1)
                                         | std::views::reverse)
        {
            statement->execute(symbols);
        }

        auto* last_statement = ast.back().get();
        if (auto expr_ptr =
                dynamic_cast<frst::ast::Expression*>(last_statement))
            fmt::println("{}",
                         expr_ptr->evaluate(symbols)->to_internal_string());
        else
            last_statement->execute(symbols);
    }
    catch (const frst::Frost_User_Error& e)
    {
        fmt::println(stderr, "Error: {}", e.what());
    }
    catch (const frst::Frost_Internal_Error& e)
    {
        fmt::println(stderr, "INTERNAL ERROR: {}", e.what());
    }
}

void repl(frst::Symbol_Table& symbols)
{
    using replxx::Replxx;

    Replxx rx;

    rx.set_unique_history(true);
    rx.enable_bracketed_paste();

    while (const char* raw_line = rx.input("\x1b[1;34m~>\x1b[0m "))
    {
        std::string line(raw_line);

        auto parse_result = frst::parse_program(line);

        if (not parse_result)
        {
            fmt::println(stderr, "{}", parse_result.error());
            continue;
        }

        if (parse_result.value().empty())
            continue;

        repl_exec(parse_result.value(), symbols);
        rx.history_add(line);
    }
}

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
        std::puts(err.what());
    }
}

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
