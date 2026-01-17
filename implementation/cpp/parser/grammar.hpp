#ifndef FROST_GRAMMAR_HPP
#define FROST_GRAMMAR_HPP

#include <charconv>

#include <frost/ast.hpp>

#include <lexy/callback.hpp>
#include <lexy/dsl.hpp>
#include <lexy/dsl/ascii.hpp>
#include <lexy/dsl/until.hpp>
#include <lexy/lexeme.hpp>
#include <lexy/token.hpp>

namespace frst::grammar
{
namespace dsl = lexy::dsl;

constexpr auto line_comment =
    dsl::token(dsl::lit_c<'#'> >> dsl::until(dsl::newline).or_eof());
constexpr auto ws = dsl::whitespace(dsl::ascii::space | line_comment);

struct identifier
{
    static constexpr auto base =
        dsl::identifier(dsl::ascii::alpha_underscore, dsl::ascii::word);

    static constexpr auto rule =
        base.reserve(LEXY_KEYWORD("if", base), LEXY_KEYWORD("else", base),
                     LEXY_KEYWORD("elif", base), LEXY_KEYWORD("def", base),
                     LEXY_KEYWORD("fn", base), LEXY_KEYWORD("reduce", base),
                     LEXY_KEYWORD("map", base), LEXY_KEYWORD("foreach", base),
                     LEXY_KEYWORD("filter", base), LEXY_KEYWORD("with", base),
                     LEXY_KEYWORD("init", base), LEXY_KEYWORD("true", base),
                     LEXY_KEYWORD("false", base), LEXY_KEYWORD("and", base),
                     LEXY_KEYWORD("or", base), LEXY_KEYWORD("not", base),
                     LEXY_KEYWORD("null", base));

    static constexpr auto value = lexy::as_string<std::string>;
};

struct integer_literal
{
    static constexpr auto rule = dsl::integer<Int>;
    static constexpr auto value = lexy::callback<Value_Ptr>([](Int value) {
        return Value::create(auto{value});
    });
};

struct float_literal : lexy::scan_production<Value_Ptr>
{
    struct out_of_range
    {
        static constexpr auto name = "float literal out of range";
    };

    template <typename Context, typename Reader>
    static constexpr scan_result scan(
        lexy::rule_scanner<Context, Reader>& scanner)
    {
        auto lexeme = scanner.capture(
            dsl::token(dsl::digits<> + dsl::lit_c<'.'> + dsl::digits<>));
        if (!lexeme)
        {
            return scan_result{};
        }

        auto text = lexeme.value();
        std::string str{text.begin(), text.end()};

        Float value{};
        auto [ptr, ec] = std::from_chars(str.data(), str.data() + str.size(),
                                         value, std::chars_format::fixed);
        if (ec == std::errc::result_out_of_range)
        {
            scanner.fatal_error(out_of_range{}, scanner.begin(),
                                scanner.position());
            return scan_result{};
        }
        if (ec != std::errc() || ptr != str.data() + str.size())
        {
            throw Frost_Internal_Error{"Failed to parse float literal"};
        }

        return Value::create(auto{value});
    }
};

struct true_literal
{
    static constexpr auto rule = LEXY_KEYWORD("true", identifier::base);
    static constexpr auto value = lexy::callback<Value_Ptr>([] {
        return Value::create(true);
    });
};

struct false_literal
{
    static constexpr auto rule = LEXY_KEYWORD("false", identifier::base);
    static constexpr auto value = lexy::callback<Value_Ptr>([] {
        return Value::create(false);
    });
};

struct bool_literal
{
    static constexpr auto rule = dsl::p<true_literal> | dsl::p<false_literal>;
    static constexpr auto value = lexy::forward<Value_Ptr>;
};

struct null_literal
{
    static constexpr auto rule = LEXY_KEYWORD("null", identifier::base);
    static constexpr auto value = lexy::callback<Value_Ptr>([] {
        return Value::null();
    });
};

struct string_literal
{
    struct raw
    {
        static constexpr auto escapes = lexy::symbol_table<char>
                                            .map<'n'>('\n')
                                            .map<'t'>('\t')
                                            .map<'r'>('\r')
                                            .map<'\\'>('\\')
                                            .map<'"'>('"');

        static constexpr auto rule = dsl::quoted.limit(dsl::ascii::newline)(
            dsl::ascii::character, dsl::backslash_escape.symbol<escapes>());
        static constexpr auto value = lexy::as_string<std::string>;
    };

    static constexpr auto rule = dsl::p<raw>;
    static constexpr auto value =
        lexy::callback<Value_Ptr>([](std::string str) {
            return Value::create(std::move(str));
        });
};

struct literal
{
    static constexpr auto rule =
        dsl::p<null_literal>
        | dsl::p<bool_literal>
        | dsl::p<string_literal>
        | (dsl::peek(dsl::token(dsl::digits<> + dsl::lit_c<'.'> + dsl::digit<>))
           >> dsl::p<float_literal>)
        | (dsl::peek(dsl::digit<>) >> dsl::p<integer_literal>);
    static constexpr auto value = lexy::forward<Value_Ptr>;
};

namespace node
{
struct Literal
{
    static constexpr auto rule = dsl::p<literal>;
    static constexpr auto value =
        lexy::new_<ast::Literal, ast::Expression::Ptr>;
};

struct Name_Lookup
{
    static constexpr auto rule = dsl::p<identifier>;
    static constexpr auto value =
        lexy::new_<ast::Name_Lookup, ast::Expression::Ptr>;
};
} // namespace node

} // namespace frst::grammar

#endif
