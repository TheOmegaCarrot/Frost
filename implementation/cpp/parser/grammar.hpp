#ifndef FROST_GRAMMAR_HPP
#define FROST_GRAMMAR_HPP

#include <charconv>
#include <vector>

#include <frost/ast.hpp>
#include <frost/lambda.hpp>

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
constexpr auto expression_start = dsl::peek(statement_ws
                                            + (dsl::ascii::alpha_underscore
                                               | dsl::digit<>
                                               | dsl::lit_c<'('>
                                               | dsl::lit_c<'['>
                                               | dsl::lit_c<'%'>
                                               | dsl::lit_c<'"'>
                                               | dsl::lit_c<'-'>));

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
struct call_arguments;
struct statement_list;

struct parenthesized_expression
{
    static constexpr auto rule = dsl::parenthesized(dsl::recurse<expression>);
    static constexpr auto value = lexy::forward<ast::Expression::Ptr>;
};

struct lambda_parameters
{
    static constexpr auto rule =
        dsl::terminator(dsl::lit_c<')'>)
            .opt_list(dsl::p<identifier>, dsl::sep(dsl::lit_c<','>));
    static constexpr auto value = [] {
        auto sink = lexy::as_list<std::vector<std::string>>;
        auto cb = lexy::callback<std::vector<std::string>>(
            [](lexy::nullopt) {
                return std::vector<std::string>{};
            },
            [](std::vector<std::string> params) {
                return params;
            });
        return sink >> cb;
    }();
};

struct lambda_body
{
    static constexpr auto rule = dsl::curly_bracketed(
        dsl::opt(expression_start >> dsl::recurse<statement_list>)
        + statement_ws);
    static constexpr auto value =
        lexy::callback<std::vector<ast::Statement::Ptr>>(
            [](lexy::nullopt) {
                return std::vector<ast::Statement::Ptr>{};
            },
            [](std::vector<ast::Statement::Ptr> body) {
                return body;
            });
};

namespace node
{
struct Lambda
{
    static constexpr auto rule = [] {
        auto kw_fn = LEXY_KEYWORD("fn", identifier::base);
        return kw_fn
               >> (dsl::lit_c<'('>
                   + dsl::p<lambda_parameters>
                   + LEXY_LIT("->")
                   + dsl::p<lambda_body>);
    }();
    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](std::vector<std::string> params,
           std::vector<ast::Statement::Ptr> body) {
            return std::make_unique<ast::Lambda>(std::move(params),
                                                 std::move(body));
        });
};

struct If_Tail
{
    static constexpr auto rule = [] {
        auto kw_elif = LEXY_KEYWORD("elif", identifier::base);
        auto kw_else = LEXY_KEYWORD("else", identifier::base);

        auto tail =
            dsl::opt(dsl::peek(kw_elif | kw_else) >> dsl::recurse<If_Tail>);

        auto elif_branch = kw_elif
                           >> (dsl::recurse<expression>
                               + dsl::lit_c<':'>
                               + dsl::recurse<expression>
                               + tail);

        auto else_branch =
            kw_else >> (dsl::lit_c<':'> + dsl::recurse<expression>);

        return elif_branch | else_branch;
    }();

    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](ast::Expression::Ptr condition, ast::Expression::Ptr consequent,
           lexy::nullopt) {
            return std::make_unique<ast::If>(std::move(condition),
                                             std::move(consequent));
        },
        [](ast::Expression::Ptr condition, ast::Expression::Ptr consequent,
           ast::Expression::Ptr alternate) {
            return std::make_unique<ast::If>(std::move(condition),
                                             std::move(consequent),
                                             std::move(alternate));
        },
        [](ast::Expression::Ptr alternate) {
            return alternate;
        });
};

struct If
{
    static constexpr auto rule = [] {
        auto kw_if = LEXY_KEYWORD("if", identifier::base);
        auto kw_elif = LEXY_KEYWORD("elif", identifier::base);
        auto kw_else = LEXY_KEYWORD("else", identifier::base);
        auto tail = dsl::opt(dsl::peek(kw_elif | kw_else) >> dsl::p<If_Tail>);
        return kw_if
               >> (dsl::recurse<expression>
                   + dsl::lit_c<':'>
                   + dsl::recurse<expression>
                   + tail);
    }();

    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](ast::Expression::Ptr condition, ast::Expression::Ptr consequent,
           lexy::nullopt) {
            return std::make_unique<ast::If>(std::move(condition),
                                             std::move(consequent));
        },
        [](ast::Expression::Ptr condition, ast::Expression::Ptr consequent,
           ast::Expression::Ptr alternate) {
            return std::make_unique<ast::If>(std::move(condition),
                                             std::move(consequent),
                                             std::move(alternate));
        });
};
} // namespace node

struct array_elements
{
    static constexpr auto rule = dsl::square_bracketed.opt_list(
        dsl::recurse<expression>, dsl::trailing_sep(dsl::lit_c<','>));
    static constexpr auto value = [] {
        auto sink = lexy::as_list<std::vector<ast::Expression::Ptr>>;
        auto cb = lexy::callback<std::vector<ast::Expression::Ptr>>(
            [](lexy::nullopt) {
                return std::vector<ast::Expression::Ptr>{};
            },
            [](std::vector<ast::Expression::Ptr> elems) {
                return elems;
            });
        return sink >> cb;
    }();
};

struct map_entry
{
    static constexpr auto rule = [] {
        auto bracket_key = dsl::square_bracketed(dsl::recurse<expression>);
        auto expr_key =
            bracket_key + dsl::lit_c<':'> + dsl::recurse<expression>;
        auto ident_key =
            dsl::p<identifier> + dsl::lit_c<':'> + dsl::recurse<expression>;
        return (dsl::peek(dsl::lit_c<'['>) >> expr_key)
               | (dsl::peek(dsl::ascii::alpha_underscore) >> ident_key);
    }();

    static constexpr auto value =
        lexy::callback<ast::Map_Constructor::KV_Pair>(
            [](ast::Expression::Ptr key, ast::Expression::Ptr value) {
                return std::make_pair(std::move(key), std::move(value));
            },
            [](std::string key, ast::Expression::Ptr value) {
                auto key_value = Value::create(std::move(key));
                auto key_expr = std::make_unique<ast::Literal>(key_value);
                return std::make_pair(std::move(key_expr), std::move(value));
            });
};

struct map_entries
{
    static constexpr auto rule =
        LEXY_LIT("%{")
        >> dsl::terminator(dsl::lit_c<'}'>)
               .opt_list(dsl::p<map_entry>,
                         dsl::trailing_sep(dsl::lit_c<','>));
    static constexpr auto value = [] {
        auto sink = lexy::as_list<std::vector<ast::Map_Constructor::KV_Pair>>;
        auto cb = lexy::callback<std::vector<ast::Map_Constructor::KV_Pair>>(
            [](lexy::nullopt) {
                return std::vector<ast::Map_Constructor::KV_Pair>{};
            },
            [](std::vector<ast::Map_Constructor::KV_Pair> elems) {
                return elems;
            });
        return sink >> cb;
    }();
};

namespace node
{
struct Array
{
    static constexpr auto rule = dsl::p<array_elements>;
    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](std::vector<ast::Expression::Ptr> elems) {
            return std::make_unique<ast::Array_Constructor>(std::move(elems));
        });
};

struct Map
{
    static constexpr auto rule = dsl::p<map_entries>;
    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](std::vector<ast::Map_Constructor::KV_Pair> pairs) {
            return std::make_unique<ast::Map_Constructor>(std::move(pairs));
        });
};
} // namespace node

struct primary_expression
{
    static constexpr auto rule =
        (dsl::peek(dsl::lit_c<'('>) >> dsl::p<parenthesized_expression>)
        | dsl::p<node::If>
        | dsl::p<node::Lambda>
        | dsl::p<node::Array>
        | dsl::p<node::Map>
        | dsl::p<node::Literal>
        | dsl::p<node::Name_Lookup>;
    static constexpr auto value = lexy::forward<ast::Expression::Ptr>;
};

struct call_arguments
{
    static constexpr auto rule =
        dsl::terminator(dsl::lit_c<')'>)
            .opt_list(dsl::recurse<expression>, dsl::sep(dsl::lit_c<','>));
    static constexpr auto value = [] {
        auto sink = lexy::as_list<std::vector<ast::Expression::Ptr>>;
        auto cb = lexy::callback<std::vector<ast::Expression::Ptr>>(
            [](lexy::nullopt) {
                return std::vector<ast::Expression::Ptr>{};
            },
            [](std::vector<ast::Expression::Ptr> args) {
                return args;
            });
        return sink >> cb;
    }();
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
    struct op_index
    {
    };
    struct op_call
    {
    };
    struct op_dot
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

    struct postfix : dsl::postfix_op
    {
        static constexpr auto op =
            dsl::op<op_index>(dsl::square_bracketed(dsl::recurse<expression>))
            / dsl::op<op_call>(dsl::lit_c<'('> >> dsl::p<call_arguments>)
            / dsl::op<op_dot>(dsl::lit_c<'.'> >> dsl::p<identifier>);
        using operand = dsl::atom;
    };

    struct prefix : dsl::prefix_op
    {
        static constexpr auto op =
            dsl::op<op_neg>(dsl::lit_c<'-'>)
            / dsl::op<op_not>(LEXY_KEYWORD("not", identifier::base));
        using operand = postfix;
    };

    struct product : dsl::infix_op_left
    {
        static constexpr auto op =
            dsl::op<op_mul>(dsl::lit_c<'*'>) / dsl::op<op_div>(dsl::lit_c<'/'>);
        using operand = prefix;
    };

    struct sum : dsl::infix_op_left
    {
        static constexpr auto op =
            dsl::op<op_add>(dsl::lit_c<'+'>) / dsl::op<op_sub>(dsl::lit_c<'-'>);
        using operand = product;
    };

    struct compare : dsl::infix_op_single
    {
        static constexpr auto op = dsl::op<op_le>(LEXY_LIT("<="))
                                   / dsl::op<op_lt>(LEXY_LIT("<"))
                                   / dsl::op<op_ge>(LEXY_LIT(">="))
                                   / dsl::op<op_gt>(LEXY_LIT(">"));
        using operand = sum;
    };

    struct equal : dsl::infix_op_single
    {
        static constexpr auto op =
            dsl::op<op_eq>(LEXY_LIT("==")) / dsl::op<op_ne>(LEXY_LIT("!="));
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
        [](ast::Expression::Ptr value) {
            return value;
        },
        [](op_neg, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Unop>(std::move(rhs), "-");
        },
        [](op_not, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Unop>(std::move(rhs), "not");
        },
        [](ast::Expression::Ptr lhs, op_index,
           ast::Expression::Ptr index_expr) {
            return std::make_unique<ast::Index>(std::move(lhs),
                                                std::move(index_expr));
        },
        [](ast::Expression::Ptr lhs, op_call,
           std::vector<ast::Expression::Ptr> args) {
            return std::make_unique<ast::Function_Call>(std::move(lhs),
                                                        std::move(args));
        },
        [](ast::Expression::Ptr lhs, op_dot, std::string key) {
            auto key_value = Value::create(std::move(key));
            auto key_expr = std::make_unique<ast::Literal>(key_value);
            return std::make_unique<ast::Index>(std::move(lhs),
                                                std::move(key_expr));
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
            return std::make_unique<ast::Binop>(std::move(lhs),
                                                "<=", std::move(rhs));
        },
        [](ast::Expression::Ptr lhs, op_gt, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs), ">",
                                                std::move(rhs));
        },
        [](ast::Expression::Ptr lhs, op_ge, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs),
                                                ">=", std::move(rhs));
        },
        [](ast::Expression::Ptr lhs, op_eq, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs),
                                                "==", std::move(rhs));
        },
        [](ast::Expression::Ptr lhs, op_ne, ast::Expression::Ptr rhs) {
            return std::make_unique<ast::Binop>(std::move(lhs),
                                                "!=", std::move(rhs));
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

namespace node
{
struct Define
{
    static constexpr auto rule = [] {
        auto kw_def = LEXY_KEYWORD("def", identifier::base);
        return kw_def
               >> (dsl::p<identifier>
                   + dsl::lit_c<'='>
                   + dsl::recurse<expression>);
    }();
    static constexpr auto value = lexy::callback<ast::Statement::Ptr>(
        [](std::string name, ast::Expression::Ptr expr) {
            return std::make_unique<ast::Define>(std::move(name),
                                                 std::move(expr));
        });
};
} // namespace node

struct expression_statement
{
    static constexpr auto rule = expression_start >> dsl::p<expression>;
    static constexpr auto value =
        lexy::callback<ast::Statement::Ptr>([](ast::Expression::Ptr expr) {
            return ast::Statement::Ptr{std::move(expr)};
        });
};

struct statement
{
    static constexpr auto rule =
        statement_ws + (dsl::p<node::Define> | dsl::p<expression_statement>);
    static constexpr auto value = lexy::forward<ast::Statement::Ptr>;
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
