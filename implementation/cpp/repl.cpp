#include <frost/ast.hpp>
#include <frost/builtin.hpp>
#include <frost/parser.hpp>
#include <frost/prelude.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <fmt/format.h>
#include <fmt/ranges.h>

#include <replxx.hxx>

#include <flat_set>
#include <stack>

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
        {
            auto val = expr_ptr->evaluate(symbols);
            if (not val->is<frst::Null>())
            {
                auto out = val->to_internal_string({.pretty = true});
                rx.write(out.data(), static_cast<int>(out.size()));
                rx.write("\n", 1);
            }
        }
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
    "if",     "else",  "elif",    "export", "def",  "fn",
    "reduce", "map",   "foreach", "filter", "with", "init",
    "true",   "false", "and",     "or",     "not",  "null",
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

struct Highlight_Callback
{

    using enum Replxx::Color;
    const static auto NUMCOLOR = YELLOW;
    const static auto STRINGCOLOR = GREEN;
    const static auto KWCOLOR = BRIGHTCYAN;

    constexpr static std::array brace_colors{RED, YELLOW, GREEN, BLUE, MAGENTA};
    std::size_t color_idx = 0;

    std::string* line = nullptr;

    struct Matched_Bracket
    {
        char open;
        char close;
        std::strong_ordering operator<=>(const Matched_Bracket&) const =
            default;
    };
    std::array<std::pair<Matched_Bracket, std::stack<Replxx::Color>>, 3>
        match_stacks{{
            {{.open = '(', .close = ')'}, {}},
            {{.open = '{', .close = '}'}, {}},
            {{.open = '[', .close = ']'}, {}},
        }};

    void reset()
    {
        for (auto& [_, stack] : match_stacks)
            stack = {};
        color_idx = 0;
    }

    void operator()(const std::string& input, Replxx::colors_t& colors)
    {
        reset();

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

        auto next_color = [&] {
            auto ret = brace_colors.at(color_idx);
            color_idx = (color_idx + 1) % brace_colors.size();
            return ret;
        };

        auto do_matches = [&](const std::string& input_segment,
                              std::size_t i) -> std::optional<Replxx::Color> {
            auto at = [&](std::size_t i) -> std::optional<char> {
                if (i < input_segment.size())
                    return input_segment[i];
                return std::nullopt;
            };

            for (auto& [match, stack] : match_stacks)
            {
                if (at(i) == match.open)
                {
                    stack.push(next_color());
                    return stack.top();
                }
                else if (at(i) == match.close)
                {
                    if (stack.empty())
                        break;
                    auto ret = stack.top();
                    stack.pop();
                    return ret;
                }
            }
            return std::nullopt;
        };

        colors.assign(input.size(), DEFAULT);

        color_idx = 0;

        if (line)
        {
            for (auto i = 0uz; i < line->size(); ++i)
            {
                do_matches(*line, i);
            }
        }

        for (auto i = 0uz; i < input.size(); ++i)
        {
            if (auto bracket_color = do_matches(input, i))
                colors[i] = *bracket_color;

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
            if (id_start(at(i)) && !id_cont(at(i - 1)))
            {
                auto start = i;
                auto end = i + 1;
                while (id_cont(at(end)))
                    ++end;
                auto substr =
                    std::string_view{input.data() + start, end - start};
                if (keywords.contains(substr))
                    for (auto brush = start; brush < end; ++brush)
                        colors[brush] = KWCOLOR;
            }
        }
    }
};

struct Completion_Callbacks
{
    std::string_view end_token(const std::string& input)
    {
        auto at = [&](std::size_t i) -> std::optional<char> {
            if (i < input.size())
                return input[i];
            return std::nullopt;
        };

        long i = input.size() - 1;
        if (!id_cont(at(i)))
            return {};

        int len = 0;
        while (i >= 0 && id_cont(at(i)))
        {
            --i;
            ++len;
        }

        if (i >= 0 && id_start(at(i)))
            ++len;

        return std::string_view{input.data() + i + 1,
                                static_cast<std::size_t>(len)};
    }

    Replxx::hints_t operator()(const std::string& input, int& len,
                               Replxx::Color& color)
    {
        std::string_view token = end_token(input);

        std::optional<std::string_view> completion;

        for (const auto& candidates : std::views::concat(
                 keywords, std::views::keys(symbols->debug_table())))
        {
            if (candidates.starts_with(token))
            {
                if (completion.has_value())
                    return {};

                completion.emplace(candidates);
            }
        }

        if (not completion)
            return {};

        if (completion->size() == token.size())
            return {};

        std::string suffix{completion->substr(token.size())};

        len = 0;
        color = Replxx::Color::BROWN;

        return {std::string{suffix}};
    }

    Replxx::completions_t operator()(const std::string& input, int& len)
    {
        std::string_view token = end_token(input);

        len = token.length();

        Replxx::completions_t out;

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
    if (input.ends_with(':') || input.ends_with("->"))
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

std::optional<std::string> read_input_segment(Replxx& rx,
                                              Highlight_Callback& hl)
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
        hl.line = &acc;
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

    Highlight_Callback highlight_callback;
    highlight_callback.reset();

    // wrap it in a lambda because replxx copies the callback
    rx.set_highlighter_callback(
        [&](const std::string& input, Replxx::colors_t& colors) {
            highlight_callback(input, colors);
        });

    Completion_Callbacks completion_callbacks{&symbols};
    rx.set_completion_callback(completion_callbacks);
    rx.set_hint_callback(completion_callbacks);

    while (auto line = read_input_segment(rx, highlight_callback))
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
        highlight_callback.reset();
    }
}
