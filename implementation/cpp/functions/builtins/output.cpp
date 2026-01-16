#include "builtins-common.hpp"

#include <fmt/core.h>

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

namespace frst
{

namespace
{

bool is_identifier_like(const String& key)
{
    if (key.empty())
    {
        return false;
    }

    auto is_alpha = [](char c) {
        return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z');
    };
    auto is_digit = [](char c) {
        return c >= '0' && c <= '9';
    };
    auto is_start = [&](char c) {
        return is_alpha(c) || c == '_';
    };
    auto is_continue = [&](char c) {
        return is_start(c) || is_digit(c);
    };

    if (!is_start(key.front()))
    {
        return false;
    }
    for (std::size_t i = 1; i < key.size(); ++i)
    {
        if (!is_continue(key.at(i)))
        {
            return false;
        }
    }

    return true;
}

std::string mformat_impl(const String& fmt_str, const Map& repl_map)
{
    std::string out;
    out.reserve(fmt_str.size());

    for (std::size_t i = 0; i < fmt_str.size();)
    {
        const auto is_dollar = fmt_str.at(i) == '$';
        const auto has_open_brace =
            (i + 1 < fmt_str.size()) && fmt_str.at(i + 1) == '{';
        const auto is_placeholder = is_dollar && has_open_brace;
        if (is_placeholder)
        {
            const auto start = i + 2;
            const auto end = fmt_str.find('}', start);
            if (end == String::npos)
            {
                throw Frost_User_Error{"Unterminated format placeholder"};
            }
            if (end == start)
            {
                throw Frost_User_Error{"Empty format placeholder"};
            }

            const auto key = fmt_str.substr(start, end - start);
            if (!is_identifier_like(key))
            {
                throw Frost_User_Error{
                    fmt::format("Invalid format placeholder: {}", key)};
            }
            const auto key_val = Value::create(auto{key});
            const auto it = repl_map.find(key_val);
            if (it == repl_map.end())
            {
                throw Frost_User_Error{
                    fmt::format("Missing replacement for key: {}", key)};
            }
            if (it->second->is<Null>())
            {
                throw Frost_User_Error{
                    fmt::format("Replacement value for key {} is null", key)};
            }

            out += it->second->to_internal_string();
            i = end + 1;
        }
        else
        {
            out += fmt_str.at(i);
            ++i;
        }
    }

    return out;
}

} // namespace

Value_Ptr mformat(builtin_args_t args)
{
    REQUIRE_ARGS(mformat, PARAM("format string", TYPES(String)),
                 PARAM("replacement map", TYPES(Map)));

    return Value::create(mformat_impl(args.at(0)->raw_get<String>(),
                                      args.at(1)->raw_get<Map>()));
}

void inject_output(Symbol_Table& table)
{
    INJECT(mformat, 2, 2);
}
} // namespace frst
