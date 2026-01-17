#ifndef FROST_GRAMMAR_HPP
#define FROST_GRAMMAR_HPP

#include <charconv>
#include <vector>

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
constexpr auto statement_ws =
    dsl::while_(dsl::ascii::space | line_comment | dsl::lit_c<';'>);

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

struct expression;

struct parenthesized_expression
{
    static constexpr auto rule = dsl::parenthesized(dsl::recurse<expression>);
    static constexpr auto value = lexy::forward<ast::Expression::Ptr>;
};

struct primary_expression
{
    static constexpr auto rule =
        (dsl::peek(dsl::lit_c<'('>) >> dsl::p<parenthesized_expression>)
        | dsl::p<node::Literal>
        | dsl::p<node::Name_Lookup>;
    static constexpr auto value = lexy::forward<ast::Expression::Ptr>;
};

struct expression : lexy::expression_production
{
    static constexpr auto whitespace = ws;
    static constexpr auto atom = dsl::p<primary_expression>;

    struct op_neg
    {
    };
    struct op_not
    {
    };
    struct op_mul
    {
    };
    struct op_div
    {
    };
    struct op_add
    {
    };
    struct op_sub
    {
    };
    struct op_lt
    {
    };
    struct op_le
    {
    };
    struct op_gt
    {
    };
    struct op_ge
    {
    };
    struct op_eq
    {
    };
    struct op_ne
    {
    };
    struct op_and
    {
    };
    struct op_or
    {
    };

    struct prefix : dsl::prefix_op
    {
        static constexpr auto op =
            dsl::op<op_neg>(dsl::lit_c<'-'>)
            / dsl::op<op_not>(LEXY_KEYWORD("not", identifier::base));
        using operand = dsl::atom;
    };

    struct product : dsl::infix_op_left
    {
        static constexpr auto op =
            dsl::op<op_mul>(dsl::lit_c<'*'>)
            / dsl::op<op_div>(dsl::lit_c<'/'>);
        using operand = prefix;
    };

    struct sum : dsl::infix_op_left
    {
        static constexpr auto op =
            dsl::op<op_add>(dsl::lit_c<'+'>)
            / dsl::op<op_sub>(dsl::lit_c<'-'>);
        using operand = product;
    };

    struct compare : dsl::infix_op_single
    {
        static constexpr auto op =
            dsl::op<op_le>(LEXY_LIT("<="))
            / dsl::op<op_lt>(LEXY_LIT("<"))
            / dsl::op<op_ge>(LEXY_LIT(">="))
            / dsl::op<op_gt>(LEXY_LIT(">"));
        using operand = sum;
    };

    struct equal : dsl::infix_op_single
    {
        static constexpr auto op =
            dsl::op<op_eq>(LEXY_LIT("=="))
            / dsl::op<op_ne>(LEXY_LIT("!="));
        using operand = compare;
    };

    struct logical_and : dsl::infix_op_left
    {
        static constexpr auto op =
            dsl::op<op_and>(LEXY_KEYWORD("and", identifier::base));
        using operand = equal;
    };

    struct logical_or : dsl::infix_op_left
    {
        static constexpr auto op =
            dsl::op<op_or>(LEXY_KEYWORD("or", identifier::base));
        using operand = logical_and;
    };

    using operation = logical_or;

    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](ast::Expression::Ptr value) { return value; },
        [](op_neg, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Unop>(std::move(rhs), "-");
        },
        [](op_not, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Unop>(std::move(rhs), "not");
        },
        [](ast::Expression::Ptr lhs, op_mul, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs), "*",
                                                std::move(rhs));
        },
        [](ast::Expression::Ptr lhs, op_div, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs), "/",
                                                std::move(rhs));
        },
        [](ast::Expression::Ptr lhs, op_add, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs), "+",
                                                std::move(rhs));
        },
        [](ast::Expression::Ptr lhs, op_sub, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs), "-",
                                                std::move(rhs));
        },
        [](ast::Expression::Ptr lhs, op_lt, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs), "<",
                                                std::move(rhs));
        },
        [](ast::Expression::Ptr lhs, op_le, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs), "<=",
                                                std::move(rhs));
        },
        [](ast::Expression::Ptr lhs, op_gt, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs), ">",
                                                std::move(rhs));
        },
        [](ast::Expression::Ptr lhs, op_ge, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs), ">=",
                                                std::move(rhs));
        },
        [](ast::Expression::Ptr lhs, op_eq, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs), "==",
                                                std::move(rhs));
        },
        [](ast::Expression::Ptr lhs, op_ne, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs), "!=",
                                                std::move(rhs));
        },
        [](ast::Expression::Ptr lhs, op_and, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs), "and",
                                                std::move(rhs));
        },
        [](ast::Expression::Ptr lhs, op_or, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs), "or",
                                                std::move(rhs));
        });
};

constexpr auto expression_start =
    dsl::peek(statement_ws + (dsl::ascii::alpha_underscore | dsl::digit<>
                              | dsl::lit_c<'('> | dsl::lit_c<'"'>
                              | dsl::lit_c<'-'>));

struct statement
{
    static constexpr auto rule = statement_ws + dsl::p<expression>;
    static constexpr auto value =
        lexy::callback<ast::Statement::Ptr>([](ast::Expression::Ptr expr) {
            return ast::Statement::Ptr{std::move(expr)};
        });
};

struct statement_list
{
    static constexpr auto rule =
        dsl::list(expression_start >> dsl::p<statement>);
    static constexpr auto value =
        lexy::as_list<std::vector<ast::Statement::Ptr>>;
};

struct program
{
    static constexpr auto whitespace = ws;
    static constexpr auto rule =
        dsl::opt(dsl::p<statement_list>) + statement_ws + dsl::eof;
    static constexpr auto value =
        lexy::callback<std::vector<ast::Statement::Ptr>>(
            [](lexy::nullopt) {
                return std::vector<ast::Statement::Ptr>{};
            },
            [](std::vector<ast::Statement::Ptr> stmts) {
                return stmts;
            });
};

} // namespace frst::grammar

#endif
