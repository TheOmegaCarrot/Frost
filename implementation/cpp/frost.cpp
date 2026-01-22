#include "frost/value.hpp"
#include <algorithm>
#include <frost/ast.hpp>
#include <frost/builtin.hpp>
#include <frost/parser.hpp>
#include <frost/symbol-table.hpp>

#include <fmt/format.h>
#include <fmt/ranges.h>

#include <replxx.hxx>

#include <flat_set>
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
        std::puts(err.what());
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

using replxx::Replxx;

void repl_exec(const std::vector<frst::ast::Statement::Ptr>& ast,
               frst::Symbol_Table& symbols, Replxx& rx)
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
            rx.print("%s\n",
                     expr_ptr->evaluate(symbols)->to_internal_string().c_str());
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

const static std::flat_set<std::string_view> keywords{
    "if",    "else",    "elif",   "def",  "fn",   "reduce",
    "map",   "foreach", "filter", "with", "init", "true",
    "false", "and",     "or",     "not",  "null",
};

bool alpha(std::optional<char> c)
{
    return c && std::isalpha(*c);
}

bool digit(std::optional<char> c)
{
    return c && std::isdigit(*c);
}

bool id_start(std::optional<char> c)
{
    return c && (*c == '_' || alpha(c));
};

bool id_cont(std::optional<char> c)
{
    return c && (*c == '_' || alpha(c) || digit(c));
};

std::optional<char> quote(std::optional<char> c)
{
    if (c == '\'' || c == '"')
        return c;
    return std::nullopt;
}

void highlight_callback(const std::string& input, Replxx::colors_t& colors)
{
    using enum Replxx::Color;
    const auto NUMCOLOR = YELLOW;
    const auto STRINGCOLOR = GREEN;
    const auto KWCOLOR = BRIGHTCYAN;

    auto at = [&](std::size_t i) -> std::optional<char> {
        if (i < input.size())
            return input[i];
        return std::nullopt;
    };

    auto in_id = [&](std::size_t i) -> bool {
        if (!id_cont(at(i)))
            return false;

        // Walk left while we are in identifier-continue chars.
        std::size_t j = i;
        while (j > 0 && id_cont(at(j - 1)))
        {
            --j;
        }

        return id_start(at(j));
    };

    colors.assign(input.size(), DEFAULT);

    for (auto i = 0uz; i < input.size(); ++i)
    {
        // numbers
        if (digit(at(i)) && not in_id(i))
            colors[i] = NUMCOLOR;

        // strings
        if (auto q = quote(at(i)))
        {
            colors[i] = STRINGCOLOR;
            ++i;
            while (at(i))
            {
                colors[i] = STRINGCOLOR;
                if (at(i) == q)
                {
                    // find out if this quote is actually escaped
                    std::size_t k = i;
                    std::size_t slashes = 0;
                    while (k > 0 && at(k - 1) == '\\')
                    {
                        --k;
                        ++slashes;
                    }
                    if (slashes % 2 == 0)
                        break; // even == escaped so ignore
                }
                ++i;
            }
        }

        // keywords
        if (id_start(at(i)))
        {
            auto start = i;
            auto end = i + 1;
            while (id_start(at(end)))
                ++end;
            auto substr = std::string_view{input.data() + start, end - start};
            if (keywords.contains(substr))
                for (auto brush = start; brush < end; ++brush)
                    colors[brush] = KWCOLOR;
        }
    }
}

struct Completion_Callback
{
    Replxx::completions_t operator()(const std::string& input, int& len)
    {
        auto at = [&](std::size_t i) -> std::optional<char> {
            if (i < input.size())
                return input[i];
            return std::nullopt;
        };

        long i = input.size() - 1;
        if (!id_cont(at(i)))
            return {};

        len = 0;
        while (i >= 0 && id_cont(at(i)))
        {
            --i;
            ++len;
        }

        if (i >= 0 && id_start(at(i)))
            ++len;

        std::string_view token{input.data() + i + 1,
                               static_cast<std::size_t>(len)};

        Replxx::completions_t out;

        // maybe: look further back if `.` before current token,
        // check if identifier before `.` is defined as a map,
        // and use any string keys as completion candidates

        for (const auto& keyword : keywords)
        {
            if (keyword.starts_with(token))
                out.emplace_back(std::string{keyword}, Replxx::Color::RED);
        }

        for (const auto& symbol : std::views::keys(symbols->debug_table()))
        {
            if (symbol.starts_with(token))
                out.emplace_back(symbol, Replxx::Color::YELLOW);
        }

        return out;
    }
    frst::Symbol_Table* symbols;
};

bool should_read_more(std::string& input)
{
    if (input.ends_with(':'))
        return true;

    if (input.ends_with('\\'))
    {
        input.pop_back();
        return true;
    }

    // intentionally signed so it can go negative on illegal syntax
    long depth = 0;
    for (char c : input)
    {
        // If they are all mismatched and wrong,
        // rejecting that is the parser's job
        if (c == '(' || c == '{' || c == '[')
            ++depth;
        if (c == ')' || c == '}' || c == ']')
            --depth;
    }

    return depth > 0;
}

std::optional<std::string> read_input_segment(Replxx& rx)
{
    const std::string main_prompt = "\x1b[1;34m~>\x1b[0m ";
    const std::string subprompt = "\x1b[1;34m..> \x1b[0m ";

    std::string acc;

    if (const char* line = rx.input(main_prompt))
        acc = line;
    else
        return std::nullopt;

    while (should_read_more(acc))
    {
        if (const char* line = rx.input(subprompt))
        {
            acc.push_back('\n');
            acc += line;
        }
        else
            break;
    }

    return acc;
}

void repl(frst::Symbol_Table& symbols)
{
    Replxx rx;

    rx.set_unique_history(true);
    rx.enable_bracketed_paste();
    rx.bind_key_internal(Replxx::KEY::control('P'), "history_previous");
    rx.bind_key_internal(Replxx::KEY::control('N'), "history_next");

    rx.set_highlighter_callback(&highlight_callback);
    rx.set_completion_callback(Completion_Callback{&symbols});

    while (auto line = read_input_segment(rx))
    {
        if (line->empty())
            continue;

        auto parse_result = frst::parse_program(*line);

        if (not parse_result)
        {
            fmt::println(stderr, "{}", parse_result.error());
            continue;
        }

        if (parse_result.value().empty())
            continue;

        repl_exec(parse_result.value(), symbols, rx);
        rx.history_add(*line);
    }
}
