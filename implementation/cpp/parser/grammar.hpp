#ifndef FROST_GRAMMAR_HPP
#define FROST_GRAMMAR_HPP

#include <charconv>
#include <optional>
#include <type_traits>
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

constexpr auto line_comment = dsl::token(
    dsl::lit_c<'#'> >> dsl::while_(dsl::ascii::character - dsl::lit_c<'\n'>));
constexpr auto no_nl_chars =
    dsl::ascii::blank | dsl::lit_c<'\r'> | dsl::lit_c<'\f'> | dsl::lit_c<'\v'>;
constexpr auto ws_no_nl = dsl::whitespace(no_nl_chars | line_comment);
constexpr auto ws_nl = dsl::whitespace(dsl::ascii::space | line_comment);
constexpr auto ws = ws_nl;
constexpr auto statement_ws = dsl::while_(
    no_nl_chars | line_comment | dsl::lit_c<';'> | dsl::lit_c<'\n'>);
constexpr auto param_ws = dsl::while_(no_nl_chars | line_comment);
constexpr auto param_ws_no_comment = dsl::while_(no_nl_chars);
constexpr auto param_ws_nl = dsl::while_(dsl::ascii::space | line_comment);
constexpr auto ws_expr_nl = ws_nl;
template <typename Ws>
constexpr auto comma_sep_ws(Ws ws)
{
    return dsl::peek(ws + dsl::lit_c<','>) >> (ws + dsl::lit_c<','> + ws);
}

constexpr auto comma_sep_nl = comma_sep_ws(param_ws_nl);
constexpr auto comma_sep = comma_sep_ws(param_ws);

template <typename After>
constexpr auto comma_sep_after(After after)
{
    return dsl::peek(param_ws + dsl::lit_c<','> + param_ws + after)
           >> (param_ws + dsl::lit_c<','> + param_ws);
}

template <typename After>
constexpr auto comma_sep_nl_after(After after)
{
    return dsl::peek(param_ws_nl + dsl::lit_c<','> + param_ws_nl + after)
           >> (param_ws_nl + dsl::lit_c<','> + param_ws_nl);
}
template <bool AllowNl>
constexpr auto expression_start_impl()
{
    auto starter = dsl::ascii::alpha_underscore
                   | dsl::digit<>
                   | dsl::lit_c<'('>
                   | dsl::lit_c<'['>
                   | dsl::lit_c<'{'>
                   | dsl::lit_c<'"'>
                   | dsl::lit_c<'\''>
                   | dsl::lit_c<'$'>
                   | dsl::lit_c<'-'>;
    if constexpr (AllowNl)
        return dsl::peek(param_ws_nl + starter);
    else
        return dsl::peek(param_ws + starter);
}

constexpr auto expression_start_no_nl = expression_start_impl<false>();
constexpr auto expression_start_nl = expression_start_impl<true>();
constexpr auto expression_start = expression_start_no_nl;

struct expected_expression
{
    static constexpr auto name = "expression";
};

struct expected_lambda_body
{
    static constexpr auto name = "lambda body";
};

struct expected_with_expression
{
    static constexpr auto name = "expression after 'with'";
};

struct expected_if_consequent
{
    static constexpr auto name = "expression after ':'";
};

struct expected_map_value
{
    static constexpr auto name = "map value";
};

struct expected_init_expression
{
    static constexpr auto name = "expression after 'init:'";
};

struct expected_call_arguments
{
    static constexpr auto name = "call arguments";
};

template <typename Expected>
constexpr auto require_expr_start_no_nl()
{
    return dsl::must(expression_start_no_nl).template error<Expected>;
}

template <typename Expected>
constexpr auto require_expr_start_nl()
{
    return dsl::must(expression_start_nl).template error<Expected>;
}

struct expected_index_expression
{
    static constexpr auto name = "index expression";
};

struct unexpected_elif
{
    static constexpr auto name = "unexpected 'elif' (missing 'if')";
};

struct unexpected_else
{
    static constexpr auto name = "unexpected 'else' (missing 'if')";
};

struct expected_vararg
{
    static constexpr auto name = "variadic parameter after ','";
};

struct expected_vararg_last
{
    static constexpr auto name = "variadic parameter must be last";
};

struct expected_destructure_binding
{
    static constexpr auto name = "destructure binding";
};

struct expected_destructure_rest
{
    static constexpr auto name = "rest binding after ','";
};

struct expected_identifier
{
    static constexpr auto name = "identifier";
};

struct identifier
{
    static constexpr auto base =
        dsl::identifier(dsl::ascii::alpha_underscore, dsl::ascii::word);

    static constexpr auto reserved = base.reserve(
        LEXY_KEYWORD("if", base), LEXY_KEYWORD("else", base),
        LEXY_KEYWORD("elif", base), LEXY_KEYWORD("def", base),
        LEXY_KEYWORD("export", base), LEXY_KEYWORD("fn", base),
        LEXY_KEYWORD("reduce", base), LEXY_KEYWORD("map", base),
        LEXY_KEYWORD("foreach", base), LEXY_KEYWORD("filter", base),
        LEXY_KEYWORD("with", base), LEXY_KEYWORD("init", base),
        LEXY_KEYWORD("true", base), LEXY_KEYWORD("false", base),
        LEXY_KEYWORD("and", base), LEXY_KEYWORD("or", base),
        LEXY_KEYWORD("not", base), LEXY_KEYWORD("null", base));

    static constexpr auto rule = reserved;

    static constexpr auto value = lexy::as_string<std::string>;
    static constexpr auto name = "identifier";
};

struct identifier_required
{
    static constexpr auto rule =
        dsl::must(dsl::peek(dsl::ascii::alpha_underscore))
            .error<expected_identifier> >> dsl::p<identifier>;
    static constexpr auto value = lexy::forward<std::string>;
    static constexpr auto name = "identifier";
};

struct integer_literal
{
    static constexpr auto rule = dsl::integer<Int>;
    static constexpr auto value = lexy::callback<Value_Ptr>([](Int value) {
        return Value::create(auto{value});
    });
    static constexpr auto name = "integer literal";
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
    static constexpr auto name = "float literal";
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
    struct raw_r_double
    {
        static constexpr auto rule =
            dsl::delimited(LEXY_LIT("R\"("), LEXY_LIT(")\""))
                .limit(dsl::ascii::newline)(dsl::ascii::character);
        static constexpr auto value = lexy::as_string<std::string>;
    };

    struct raw_r_single
    {
        static constexpr auto rule =
            dsl::delimited(LEXY_LIT("R'("), LEXY_LIT(")'"))
                .limit(dsl::ascii::newline)(dsl::ascii::character);
        static constexpr auto value = lexy::as_string<std::string>;
    };

    struct raw_double
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

    struct raw_single
    {
        static constexpr auto escapes = lexy::symbol_table<char>
                                            .map<'n'>('\n')
                                            .map<'t'>('\t')
                                            .map<'r'>('\r')
                                            .map<'\\'>('\\')
                                            .map<'\''>('\'');

        static constexpr auto rule =
            dsl::single_quoted.limit(dsl::ascii::newline)(
                dsl::ascii::character, dsl::backslash_escape.symbol<escapes>());
        static constexpr auto value = lexy::as_string<std::string>;
    };

    static constexpr auto rule = dsl::p<raw_r_double>
                                 | dsl::p<raw_r_single>
                                 | dsl::p<raw_double>
                                 | dsl::p<raw_single>;
    static constexpr auto value =
        lexy::callback<Value_Ptr>([](std::string str) {
            return Value::create(std::move(str));
        });
    static constexpr auto name = "string literal";
};

struct format_string_literal
{
    static constexpr auto rule =
        dsl::p<string_literal::raw_double> | dsl::p<string_literal::raw_single>;
    static constexpr auto value = lexy::forward<std::string>;
    static constexpr auto name = "format string literal";
};

namespace node
{
struct Literal
{
    static constexpr auto rule =
        dsl::p<null_literal>
        | dsl::p<bool_literal>
        | dsl::p<string_literal>
        | (dsl::peek(dsl::token(dsl::digits<> + dsl::lit_c<'.'> + dsl::digit<>))
           >> dsl::p<float_literal>)
        | (dsl::peek(dsl::digit<>) >> dsl::p<integer_literal>);
    static constexpr auto value =
        lexy::new_<ast::Literal, ast::Expression::Ptr>;
    static constexpr auto name = "literal";
};

struct Format_String
{
    static constexpr auto rule =
        dsl::no_whitespace(dsl::lit_c<'$'> >> dsl::p<format_string_literal>);
    static constexpr auto value =
        lexy::callback<ast::Expression::Ptr>([](std::string format) {
            return std::make_unique<ast::Format_String>(std::move(format));
        });
    static constexpr auto name = "format string";
};

struct Name_Lookup
{
    static constexpr auto rule = dsl::p<identifier>;
    static constexpr auto value =
        lexy::new_<ast::Name_Lookup, ast::Expression::Ptr>;
    static constexpr auto name = "name";
};
} // namespace node

struct expression;
struct expression_nl;
struct statement_list;

template <typename T>
constexpr auto list_or_empty()
{
    auto sink = lexy::as_list<std::vector<T>>;
    auto cb = lexy::callback<std::vector<T>>(
        [](lexy::nullopt) {
            return std::vector<T>{};
        },
        [](std::vector<T> items) {
            return items;
        });
    return sink >> cb;
}

template <typename EntryStart, typename List, typename Close>
constexpr auto delimited_list_tail(EntryStart entry_start, List list,
                                   Close close)
{
    return param_ws_nl
           + dsl::opt(dsl::peek(entry_start) >> list)
           + param_ws_nl
           + close;
}

template <typename Open, typename EntryStart, typename List, typename Close>
constexpr auto delimited_list(Open open, EntryStart entry_start, List list,
                              Close close)
{
    return open + delimited_list_tail(entry_start, list, close);
}

template <typename EntryStart, typename List>
constexpr auto braced_list(EntryStart entry_start, List list)
{
    return delimited_list(LEXY_LIT("{"), entry_start, list, dsl::lit_c<'}'>);
}

template <typename OpIndex>
constexpr auto index_postfix_op()
{
    return dsl::op<OpIndex>(
        dsl::
            lit_c<'['> >> (param_ws_nl
                           + (require_expr_start_nl<expected_index_expression>()
                              >> dsl::recurse<expression_nl>)+param_ws_nl
                           + dsl::lit_c<']'>));
}

template <typename OpDot>
constexpr auto dot_postfix_op()
{
    return dsl::op<OpDot>(dsl::lit_c<'.'> >> dsl::p<identifier_required>);
}

struct lambda_param_pack
{
    std::vector<std::string> params;
    std::optional<std::string> vararg;
};

struct destructure_pack
{
    std::vector<ast::Array_Destructure::Name> names;
    std::optional<ast::Array_Destructure::Name> rest;
};

struct destructure_binding
{
    static constexpr auto rule = dsl::p<identifier_required>;
    static constexpr auto value =
        lexy::callback<ast::Array_Destructure::Name>([](std::string name) {
            if (name == "_")
                return ast::Array_Destructure::Name{ast::Discarded_Binding{}};
            return ast::Array_Destructure::Name{std::move(name)};
        });
    static constexpr auto name = "destructure binding";
};

struct destructure_binding_list
{
    static constexpr auto rule = [] {
        auto elem_start = dsl::ascii::alpha_underscore;
        auto sep = comma_sep_nl_after(elem_start);
        return dsl::list(dsl::p<destructure_binding>, dsl::sep(sep));
    }();
    static constexpr auto value =
        lexy::as_list<std::vector<ast::Array_Destructure::Name>>;
    static constexpr auto name = "destructure bindings";
};

struct destructure_payload
{
    static constexpr auto rule = [] {
        auto leading_comma = dsl::peek(dsl::lit_c<','>)
                             >> dsl::error<expected_destructure_binding>;
        auto rest = (dsl::must(dsl::peek(LEXY_LIT("...")))
                         .error<
                             expected_destructure_rest
                         > >> (LEXY_LIT("...")
                               + param_ws_nl
                               + dsl::p<destructure_binding>));
        auto rest_checked = rest
                            + dsl::must(dsl::peek_not(dsl::lit_c<','>))
                                  .error<expected_vararg_last>;
        auto names = dsl::p<destructure_binding_list>;
        auto names_then_rest =
            names
            + dsl::opt(dsl::peek(dsl::lit_c<','>)
                       >> (dsl::lit_c<','> + param_ws_nl + rest_checked));
        auto rest_only = dsl::peek(LEXY_LIT("...")) >> rest_checked;
        auto names_only =
            dsl::peek(dsl::ascii::alpha_underscore) >> names_then_rest;
        return dsl::opt(rest_only | names_only | leading_comma);
    }();

    static constexpr auto value = lexy::callback<destructure_pack>(
        [](lexy::nullopt) {
            return destructure_pack{};
        },
        [](ast::Array_Destructure::Name rest) {
            return destructure_pack{{}, std::move(rest)};
        },
        [](std::vector<ast::Array_Destructure::Name> names, lexy::nullopt) {
            return destructure_pack{std::move(names), std::nullopt};
        },
        [](std::vector<ast::Array_Destructure::Name> names,
           ast::Array_Destructure::Name rest) {
            return destructure_pack{std::move(names), std::move(rest)};
        });
    static constexpr auto name = "destructure payload";
};

struct array_destructure_pattern
{
    static constexpr auto rule = dsl::lit_c<'['>
                                 + param_ws_nl
                                 + dsl::p<destructure_payload>
                                 + param_ws_nl
                                 + dsl::lit_c<']'>;
    static constexpr auto value = lexy::forward<destructure_pack>;
    static constexpr auto name = "array destructure";
};

template <bool AllowNl>
struct lambda_param_list_impl
{
    static constexpr auto rule = [] {
        if constexpr (AllowNl)
        {
            auto param_sep = comma_sep_nl_after(dsl::p<identifier>);
            return dsl::list(dsl::p<identifier_required>, dsl::sep(param_sep));
        }
        else
        {
            auto param_sep = comma_sep_after(dsl::p<identifier>);
            return dsl::list(dsl::p<identifier_required>, dsl::sep(param_sep));
        }
    }();

    static constexpr auto value = lexy::as_list<std::vector<std::string>>;
    static constexpr auto name = "lambda parameters";
};

using lambda_param_list = lambda_param_list_impl<false>;
using lambda_param_list_nl = lambda_param_list_impl<true>;

template <bool AllowNl>
struct lambda_param_payload_impl
{
    static constexpr auto rule = [] {
        auto vararg_name = [] {
            if constexpr (AllowNl)
                return LEXY_LIT("...")
                       + param_ws_nl
                       + dsl::p<identifier_required>;
            else
                return LEXY_LIT("...") + dsl::p<identifier_required>;
        }();
        auto vararg = (dsl::must(dsl::peek(LEXY_LIT("...")))
                           .template error<expected_vararg> >> vararg_name);
        auto vararg_checked = vararg
                              + dsl::must(dsl::peek_not(dsl::lit_c<','>))
                                    .template error<expected_vararg_last>;
        auto params = dsl::p<lambda_param_list_impl<AllowNl>>;
        auto params_then_vararg = [&] {
            if constexpr (AllowNl)
            {
                return params
                       + dsl::opt(dsl::peek(param_ws_nl + dsl::lit_c<','>)
                                  >> (param_ws_nl
                                      + dsl::lit_c<','>
                                      + param_ws_nl
                                      + vararg_checked));
            }
            else
            {
                return params
                       + dsl::opt(dsl::peek(dsl::lit_c<','>)
                                  >> (dsl::lit_c<','> + vararg_checked));
            }
        }();
        auto vararg_only = dsl::peek(LEXY_LIT("...")) >> vararg_checked;
        auto params_only =
            dsl::peek(dsl::ascii::alpha_underscore) >> params_then_vararg;
        return dsl::opt(vararg_only | params_only);
    }();

    static constexpr auto value = lexy::callback<lambda_param_pack>(
        [](lexy::nullopt) {
            return lambda_param_pack{};
        },
        [](std::string vararg) {
            return lambda_param_pack{{}, std::move(vararg)};
        },
        [](std::vector<std::string> params, lexy::nullopt) {
            return lambda_param_pack{std::move(params), std::nullopt};
        },
        [](std::vector<std::string> params, std::string vararg) {
            return lambda_param_pack{std::move(params), std::move(vararg)};
        });
    static constexpr auto name = "lambda parameters";
};

using lambda_param_payload = lambda_param_payload_impl<false>;
using lambda_param_payload_nl = lambda_param_payload_impl<true>;

struct lambda_parameters_paren
{
    static constexpr auto rule = dsl::lit_c<'('>
                                 + param_ws_nl
                                 + dsl::p<lambda_param_payload_nl>
                                 + param_ws_nl
                                 + dsl::lit_c<')'>;
    static constexpr auto value = lexy::forward<lambda_param_pack>;
    static constexpr auto name = "lambda parameters";
};

struct lambda_parameters_elided
{
    static constexpr auto rule = dsl::p<lambda_param_payload>;
    static constexpr auto value = lexy::forward<lambda_param_pack>;
    static constexpr auto name = "lambda parameters";
};

struct lambda_param_clause
{
    static constexpr auto rule = [] {
        auto kw_arrow = LEXY_LIT("->");
        auto params = dsl::peek(param_ws_nl + dsl::lit_c<'('>)
                      >> (param_ws_nl + dsl::p<lambda_parameters_paren>)
                      | dsl::else_
                      >> (param_ws_nl + dsl::p<lambda_parameters_elided>);
        return params + param_ws_nl + kw_arrow;
    }();
    static constexpr auto value = lexy::forward<lambda_param_pack>;
    static constexpr auto name = "lambda parameters";
};

struct lambda_body
{
    static constexpr auto rule =
        dsl::peek(param_ws_nl + dsl::lit_c<'{'>)
        >> param_ws_nl
        >> dsl::curly_bracketed(statement_ws
                                + dsl::opt(dsl::peek(expression_start_no_nl)
                                           >> dsl::recurse<statement_list>)
                                + statement_ws)
        | dsl::else_
        >> (require_expr_start_nl<expected_lambda_body>()
            >> param_ws_nl
            >> dsl::recurse<expression>);
    static constexpr auto value =
        lexy::callback<std::vector<ast::Statement::Ptr>>(
            [](lexy::nullopt) {
                return std::vector<ast::Statement::Ptr>{};
            },
            [](std::vector<ast::Statement::Ptr> stmts) {
                return stmts;
            },
            [](ast::Expression::Ptr expr) {
                std::vector<ast::Statement::Ptr> stmts;
                stmts.emplace_back(ast::Statement::Ptr{std::move(expr)});
                return stmts;
            });
    static constexpr auto name = "lambda body";
};

namespace node
{
constexpr auto kw_with = LEXY_KEYWORD("with", identifier::base);

template <typename Node>
constexpr const char* with_operation_name()
{
    if constexpr (std::is_same_v<Node, ast::Map>)
        return "map expression";
    if constexpr (std::is_same_v<Node, ast::Filter>)
        return "filter expression";
    if constexpr (std::is_same_v<Node, ast::Foreach>)
        return "foreach expression";
    return "with expression";
}

template <typename Node>
constexpr auto with_operation_keyword()
{
    if constexpr (std::is_same_v<Node, ast::Map>)
        return LEXY_KEYWORD("map", identifier::base);
    if constexpr (std::is_same_v<Node, ast::Filter>)
        return LEXY_KEYWORD("filter", identifier::base);
    if constexpr (std::is_same_v<Node, ast::Foreach>)
        return LEXY_KEYWORD("foreach", identifier::base);
}

template <typename Node>
struct With_Operation
{
    static constexpr auto rule =
        dsl::recurse<expression>
        + param_ws_nl
        + kw_with
        + param_ws_nl
        + (require_expr_start_nl<expected_with_expression>()
           >> dsl::recurse<expression>);
    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](ast::Expression::Ptr structure, ast::Expression::Ptr operation) {
            return std::make_unique<Node>(std::move(structure),
                                          std::move(operation));
        });
    static constexpr auto name = with_operation_name<Node>();
};

template <typename Node>
struct With_Keyword_Expr
{
    static constexpr auto rule = [] {
        auto kw = with_operation_keyword<Node>();
        return kw + param_ws_nl + dsl::p<With_Operation<Node>>;
    }();
    static constexpr auto value = lexy::forward<ast::Expression::Ptr>;
    static constexpr auto name = with_operation_name<Node>();
};

using Map_Expr = With_Keyword_Expr<ast::Map>;
using Filter = With_Keyword_Expr<ast::Filter>;
using Foreach = With_Keyword_Expr<ast::Foreach>;

struct Reduce
{
    static constexpr auto rule = [] {
        auto kw_reduce = LEXY_KEYWORD("reduce", identifier::base);
        auto kw_with = LEXY_KEYWORD("with", identifier::base);
        auto kw_init = LEXY_KEYWORD("init", identifier::base);
        auto init_clause =
            dsl::opt(dsl::peek(param_ws_nl + kw_init)
                     >> (param_ws_nl
                         + kw_init
                         + param_ws_nl
                         + dsl::lit_c<':'>
                         + param_ws_nl
                         + (require_expr_start_no_nl<expected_init_expression>()
                            >> dsl::recurse<expression>)));
        return kw_reduce
               >> (param_ws_nl
                   + dsl::recurse<expression>
                   + param_ws_nl
                   + kw_with
                   + param_ws_nl
                   + (require_expr_start_no_nl<expected_with_expression>()
                      >> dsl::recurse<expression>)+init_clause);
    }();
    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](ast::Expression::Ptr structure, ast::Expression::Ptr operation,
           lexy::nullopt) {
            return std::make_unique<ast::Reduce>(
                std::move(structure), std::move(operation),
                std::optional<ast::Expression::Ptr>{});
        },
        [](ast::Expression::Ptr structure, ast::Expression::Ptr operation,
           ast::Expression::Ptr init) {
            return std::make_unique<ast::Reduce>(
                std::move(structure), std::move(operation),
                std::optional<ast::Expression::Ptr>{std::move(init)});
        });
    static constexpr auto name = "reduce expression";
};

struct Lambda
{
    static constexpr auto rule = [] {
        auto kw_fn = LEXY_KEYWORD("fn", identifier::base);
        return kw_fn >> (dsl::p<lambda_param_clause> + dsl::p<lambda_body>);
    }();
    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](lambda_param_pack params, std::vector<ast::Statement::Ptr> body) {
            return std::make_unique<ast::Lambda>(std::move(params.params),
                                                 std::move(body),
                                                 std::move(params.vararg));
        });
    static constexpr auto name = "lambda expression";
};

struct If
{
    struct Tail
    {
        static constexpr auto rule = [] {
            auto kw_elif = LEXY_KEYWORD("elif", identifier::base);
            auto kw_else = LEXY_KEYWORD("else", identifier::base);

            auto tail = dsl::opt(dsl::peek(param_ws_nl + (kw_elif | kw_else))
                                 >> param_ws_nl
                                 >> dsl::recurse<Tail>);

            auto elif_branch =
                kw_elif
                >> (dsl::recurse<expression>
                    + dsl::lit_c<':'>
                    + param_ws_nl
                    + (require_expr_start_no_nl<expected_if_consequent>()
                       >> dsl::recurse<expression>)+tail);

            auto else_branch =
                kw_else
                >> (dsl::lit_c<':'>
                    + param_ws_nl
                    + (require_expr_start_no_nl<expected_if_consequent>()
                       >> dsl::recurse<expression>));

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
        static constexpr auto name = "if expression tail";
    };

    static constexpr auto rule = [] {
        auto kw_if = LEXY_KEYWORD("if", identifier::base);
        auto kw_elif = LEXY_KEYWORD("elif", identifier::base);
        auto kw_else = LEXY_KEYWORD("else", identifier::base);

        auto tail = dsl::opt(dsl::peek(param_ws_nl + (kw_elif | kw_else))
                             >> param_ws_nl
                             >> dsl::p<Tail>);
        return kw_if
               >> (dsl::recurse<expression>
                   + dsl::lit_c<':'>
                   + param_ws_nl
                   + (require_expr_start_no_nl<expected_if_consequent>()
                      >> dsl::recurse<expression>)+tail);
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
    static constexpr auto name = "if expression";
};
} // namespace node

inline ast::Expression::Ptr make_string_key_expr(std::string key)
{
    auto key_value = Value::create(std::move(key));
    return std::make_unique<ast::Literal>(std::move(key_value));
}

struct array_elements
{
    static constexpr auto rule = [] {
        auto item =
            dsl::peek(expression_start_nl) >> dsl::recurse<expression_nl>;
        auto list = dsl::list(item, dsl::trailing_sep(comma_sep_nl));
        return delimited_list(dsl::lit_c<'['>, expression_start_nl, list,
                              dsl::lit_c<']'>);
    }();
    static constexpr auto value = list_or_empty<ast::Expression::Ptr>();
    static constexpr auto name = "array literal";
};

constexpr auto map_entry_start = dsl::lit_c<'['> | dsl::ascii::alpha_underscore;

struct map_key
{
    static constexpr auto rule = [] {
        auto bracket_key = dsl::lit_c<'['>
                           + param_ws_nl
                           + dsl::recurse<expression_nl>
                           + param_ws_nl
                           + dsl::lit_c<']'>;
        auto ident_key = dsl::p<identifier_required>;
        auto key =
            dsl::peek(dsl::lit_c<'['>) >> bracket_key | dsl::else_ >> ident_key;
        return key;
    }();

    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](ast::Expression::Ptr key) {
            return key;
        },
        [](std::string key) {
            return make_string_key_expr(std::move(key));
        });
    static constexpr auto name = "map key";
};

template <typename ValueRule>
constexpr auto map_entry_rule(ValueRule value_rule)
{
    return dsl::p<map_key>
           + param_ws_nl
           + dsl::lit_c<':'>
           + param_ws_nl
           + value_rule;
}

struct map_entry
{
    static constexpr auto rule = [] {
        auto value = require_expr_start_nl<expected_map_value>()
                     >> dsl::recurse<expression_nl>;
        return map_entry_rule(value);
    }();

    static constexpr auto value = lexy::callback<ast::Map_Constructor::KV_Pair>(
        [](ast::Expression::Ptr key, ast::Expression::Ptr value) {
            return std::make_pair(std::move(key), std::move(value));
        });
    static constexpr auto name = "map entry";
};

struct map_destructure_entry
{
    static constexpr auto rule = [] {
        return map_entry_rule(dsl::p<identifier_required>);
    }();

    static constexpr auto value = lexy::callback<ast::Map_Destructure::Element>(
        [](ast::Expression::Ptr key, std::string name) {
            return ast::Map_Destructure::Element{std::move(key),
                                                 std::move(name)};
        });
    static constexpr auto name = "map destructure entry";
};

struct map_entries
{
    static constexpr auto rule = [] {
        auto entry_start = dsl::peek(map_entry_start);
        auto item = entry_start >> dsl::p<map_entry>;
        auto list = dsl::list(item, dsl::trailing_sep(comma_sep_nl));
        return braced_list(map_entry_start, list);
    }();
    static constexpr auto value =
        list_or_empty<ast::Map_Constructor::KV_Pair>();
    static constexpr auto name = "map literal";
};

struct map_destructure_entries
{
    static constexpr auto rule = [] {
        auto entry_start = dsl::peek(map_entry_start);
        auto item = entry_start >> dsl::p<map_destructure_entry>;
        auto sep = comma_sep_nl_after(map_entry_start);
        auto list = dsl::list(item, dsl::sep(sep));
        return braced_list(map_entry_start, list);
    }();
    static constexpr auto value =
        list_or_empty<ast::Map_Destructure::Element>();
    static constexpr auto name = "map destructure";
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
    static constexpr auto name = "array literal";
};

struct Map
{
    static constexpr auto rule = dsl::p<map_entries>;
    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](std::vector<ast::Map_Constructor::KV_Pair> pairs) {
            return std::make_unique<ast::Map_Constructor>(std::move(pairs));
        });
    static constexpr auto name = "map literal";
};
} // namespace node

struct primary_expression
{
    static constexpr auto rule =
        (dsl::peek(dsl::lit_c<'('>)
         >> (dsl::lit_c<'('>
             + param_ws_nl
             + dsl::recurse<expression_nl>
             + param_ws_nl
             + dsl::lit_c<')'>))
        | dsl::peek(LEXY_KEYWORD("if", identifier::base))
        >> dsl::p<node::If>
        | dsl::peek(LEXY_KEYWORD("fn", identifier::base))
        >> dsl::p<node::Lambda>
        | dsl::peek(LEXY_KEYWORD("map", identifier::base))
        >> dsl::p<node::Map_Expr>
        | dsl::peek(LEXY_KEYWORD("filter", identifier::base))
        >> dsl::p<node::Filter>
        | dsl::peek(LEXY_KEYWORD("reduce", identifier::base))
        >> dsl::p<node::Reduce>
        | dsl::peek(LEXY_KEYWORD("foreach", identifier::base))
        >> dsl::p<node::Foreach>
        | dsl::peek(dsl::lit_c<'['>)
        >> dsl::p<node::Array>
        | dsl::peek(dsl::lit_c<'{'>)
        >> dsl::p<node::Map>
        | dsl::peek(dsl::lit_c<'$'>)
        >> dsl::p<node::Format_String>
        | dsl::p<node::Literal>
        | dsl::peek(LEXY_KEYWORD("elif", identifier::base))
        >> dsl::error<unexpected_elif>
        | dsl::peek(LEXY_KEYWORD("else", identifier::base))
        >> dsl::error<unexpected_else>
        | dsl::p<node::Name_Lookup>
        | dsl::error<expected_expression>;
    static constexpr auto value = lexy::forward<ast::Expression::Ptr>;
    static constexpr auto name = "expression";
};

struct call_arguments
{
    static constexpr auto rule = [] {
        auto list =
            dsl::list(dsl::recurse<expression_nl>, dsl::sep(comma_sep_nl));
        return delimited_list_tail(expression_start_nl, list, dsl::lit_c<')'>);
    }();
    static constexpr auto value = list_or_empty<ast::Expression::Ptr>();
    static constexpr auto name = "call arguments";
};

template <bool AllowNl>
struct expression_impl : lexy::expression_production
{
    static constexpr auto whitespace = [] {
        if constexpr (AllowNl)
            return ws_expr_nl;
        else
            return ws_no_nl;
    }();
    static constexpr auto atom = dsl::p<primary_expression>;
    static constexpr auto name = "expression";

    struct op_neg
    {
        static constexpr ast::Unary_Op op = ast::Unary_Op::NEGATE;
    };
    struct op_not
    {
        static constexpr ast::Unary_Op op = ast::Unary_Op::NOT;
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
    struct op_ufcs
    {
    };
    struct op_mod
    {
        static constexpr ast::Binary_Op op = ast::Binary_Op::MODULUS;
    };
    struct op_mul
    {
        static constexpr ast::Binary_Op op = ast::Binary_Op::MULTIPLY;
    };
    struct op_div
    {
        static constexpr ast::Binary_Op op = ast::Binary_Op::DIVIDE;
    };
    struct op_add
    {
        static constexpr ast::Binary_Op op = ast::Binary_Op::PLUS;
    };
    struct op_sub
    {
        static constexpr ast::Binary_Op op = ast::Binary_Op::MINUS;
    };
    struct op_lt
    {
        static constexpr ast::Binary_Op op = ast::Binary_Op::LT;
    };
    struct op_le
    {
        static constexpr ast::Binary_Op op = ast::Binary_Op::LE;
    };
    struct op_gt
    {
        static constexpr ast::Binary_Op op = ast::Binary_Op::GT;
    };
    struct op_ge
    {
        static constexpr ast::Binary_Op op = ast::Binary_Op::GE;
    };
    struct op_eq
    {
        static constexpr ast::Binary_Op op = ast::Binary_Op::EQ;
    };
    struct op_ne
    {
        static constexpr ast::Binary_Op op = ast::Binary_Op::NE;
    };
    struct op_and
    {
        static constexpr ast::Binary_Op op = ast::Binary_Op::AND;
    };
    struct op_or
    {
        static constexpr ast::Binary_Op op = ast::Binary_Op::OR;
    };

    struct ufcs_call
    {
        struct callee : lexy::expression_production
        {
            static constexpr auto whitespace = [] {
                if constexpr (AllowNl)
                    return ws_nl;
                else
                    return ws_no_nl;
            }();
            static constexpr auto atom = dsl::p<primary_expression>;

            struct postfix : dsl::postfix_op
            {
                static constexpr auto op =
                    index_postfix_op<op_index>() / dot_postfix_op<op_dot>();
                using operand = dsl::atom;
            };

            using operation = postfix;

            static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
                [](ast::Expression::Ptr value) {
                    return value;
                },
                [](ast::Expression::Ptr lhs, op_index,
                   ast::Expression::Ptr index_expr) {
                    return std::make_unique<ast::Index>(std::move(lhs),
                                                        std::move(index_expr));
                },
                [](ast::Expression::Ptr lhs, op_dot, std::string key) {
                    return std::make_unique<ast::Index>(
                        std::move(lhs), make_string_key_expr(std::move(key)));
                });
        };

        struct result
        {
            ast::Expression::Ptr callee;
            std::vector<ast::Expression::Ptr> args;
        };

        static constexpr auto rule =
            dsl::p<callee> + dsl::lit_c<'('> + dsl::p<call_arguments>;
        static constexpr auto value =
            lexy::callback<result>([](ast::Expression::Ptr callee,
                                      std::vector<ast::Expression::Ptr> args) {
                return result{std::move(callee), std::move(args)};
            });
        static constexpr auto name = "UFCS call";
    };

    struct postfix : dsl::postfix_op
    {
        static constexpr auto op =
            index_postfix_op<op_index>()
            / dsl::op<op_call>(
                dsl::lit_c<'('> >> (dsl::must(
                                        dsl::peek(param_ws_nl + dsl::lit_c<')'>)
                                        | expression_start_nl)
                                        .error<expected_call_arguments> >> dsl::
                                        p<call_arguments>))
            / dot_postfix_op<op_dot>()
            / dsl::op<op_ufcs>(dsl::lit_c<'@'> >> dsl::p<ufcs_call>);
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
        static constexpr auto op = dsl::op<op_mod>(dsl::lit_c<'%'>)
                                   / dsl::op<op_mul>(dsl::lit_c<'*'>)
                                   / dsl::op<op_div>(dsl::lit_c<'/'>);
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
        []<typename Op>(Op, ast::Expression::Ptr rhs)
            requires requires {
                { Op::op } -> std::convertible_to<ast::Unary_Op>;
            }
                     {
                         return std::make_unique<ast::Unop>(std::move(rhs),
                                                            Op::op);
                     },
                     [](ast::Expression::Ptr lhs, op_index,
                        ast::Expression::Ptr index_expr) {
                         return std::make_unique<ast::Index>(
                             std::move(lhs), std::move(index_expr));
                     },
                     [](ast::Expression::Ptr lhs, op_call,
                        std::vector<ast::Expression::Ptr> args) {
                         return std::make_unique<ast::Function_Call>(
                             std::move(lhs), std::move(args));
                     },
                     [](ast::Expression::Ptr lhs, op_dot, std::string key) {
                         return std::make_unique<ast::Index>(
                             std::move(lhs),
                             make_string_key_expr(std::move(key)));
                     },
                     [](ast::Expression::Ptr lhs, op_ufcs,
                        ufcs_call::result rhs) {
                         auto args = std::move(rhs.args);
                         args.insert(args.begin(), std::move(lhs));
                         return std::make_unique<ast::Function_Call>(
                             std::move(rhs.callee), std::move(args));
                     },
                     []<typename Op>(ast::Expression::Ptr lhs, Op,
                                     ast::Expression::Ptr rhs)
                         requires requires {
                             { Op::op } -> std::convertible_to<ast::Binary_Op>;
                         }
        {
            return std::make_unique<ast::Binop>(std::move(lhs), Op::op,
                                                std::move(rhs));
        });
};

struct expression : expression_impl<false>
{
};

struct expression_nl : expression_impl<true>
{
};

namespace node
{
constexpr auto define_lhs = [] {
    return dsl::peek(dsl::lit_c<'['>)
           >> dsl::p<array_destructure_pattern>
           | dsl::peek(dsl::lit_c<'{'>)
           >> dsl::p<map_destructure_entries>
           | dsl::else_
           >> dsl::p<identifier_required>;
}();

constexpr auto define_payload = [] {
    return define_lhs + dsl::lit_c<'='> + param_ws + dsl::p<expression>;
}();

template <bool Export>
constexpr auto define_callback()
{
    return lexy::callback<ast::Statement::Ptr>(
        [](std::string name, ast::Expression::Ptr expr) {
            return std::make_unique<ast::Define>(std::move(name),
                                                 std::move(expr), Export);
        },
        [](std::string name, lexy::nullopt, ast::Expression::Ptr expr) {
            return std::make_unique<ast::Define>(std::move(name),
                                                 std::move(expr), Export);
        },
        [](destructure_pack pack, ast::Expression::Ptr expr) {
            return std::make_unique<ast::Array_Destructure>(
                std::move(pack.names), std::move(pack.rest), std::move(expr),
                Export);
        },
        [](std::vector<ast::Map_Destructure::Element> elems,
           ast::Expression::Ptr expr) {
            return std::make_unique<ast::Map_Destructure>(
                std::move(elems), std::move(expr), Export);
        },
        [](destructure_pack pack, lexy::nullopt, ast::Expression::Ptr expr) {
            return std::make_unique<ast::Array_Destructure>(
                std::move(pack.names), std::move(pack.rest), std::move(expr),
                Export);
        },
        [](std::vector<ast::Map_Destructure::Element> elems, lexy::nullopt,
           ast::Expression::Ptr expr) {
            return std::make_unique<ast::Map_Destructure>(
                std::move(elems), std::move(expr), Export);
        });
}

struct Define
{
    static constexpr auto rule = [] {
        auto kw_def = LEXY_KEYWORD("def", identifier::base);
        return kw_def >> define_payload;
    }();
    static constexpr auto value = define_callback<false>();
    static constexpr auto name = "def statement";
};

struct Export_Def
{
    static constexpr auto rule = [] {
        auto kw_export = LEXY_KEYWORD("export", identifier::base);
        auto kw_def = LEXY_KEYWORD("def", identifier::base);
        return kw_export + param_ws_no_comment + kw_def + define_payload;
    }();
    static constexpr auto value = define_callback<true>();
    static constexpr auto name = "export def statement";
};
} // namespace node

struct expression_statement
{
    static constexpr auto rule = expression_start_no_nl >> dsl::p<expression>;
    static constexpr auto value =
        lexy::callback<ast::Statement::Ptr>([](ast::Expression::Ptr expr) {
            return ast::Statement::Ptr{std::move(expr)};
        });
    static constexpr auto name = "expression statement";
};

template <bool AllowExport>
struct statement_impl
{
    static constexpr auto rule = [] {
        if constexpr (AllowExport)
        {
            auto kw_export = LEXY_KEYWORD("export", identifier::base);
            return param_ws
                   + ((dsl::peek(kw_export) >> dsl::p<node::Export_Def>)
                      | dsl::p<node::Define>
                      | dsl::p<expression_statement>);
        }
        else
        {
            return param_ws
                   + (dsl::p<node::Define> | dsl::p<expression_statement>);
        }
    }();
    static constexpr auto value = lexy::forward<ast::Statement::Ptr>;
    static constexpr auto name = "statement";
};

using statement = statement_impl<false>;
using top_level_statement = statement_impl<true>;

template <typename Stmt>
struct statement_list_impl
{
    static constexpr auto rule = [] {
        auto sep =
            dsl::peek(dsl::while_(no_nl_chars)
                      + (dsl::lit_c<';'> | dsl::lit_c<'\n'> | dsl::lit_c<'#'>))
            >> statement_ws;
        auto item = dsl::peek(expression_start_no_nl) >> dsl::p<Stmt>;
        return dsl::peek(expression_start_no_nl)
               >> dsl::list(item, dsl::trailing_sep(sep));
    }();
    static constexpr auto value =
        lexy::as_list<std::vector<ast::Statement::Ptr>>;
    static constexpr auto name = "statement list";
};

struct statement_list : statement_list_impl<statement>
{
};

struct top_level_statement_list : statement_list_impl<top_level_statement>
{
};

struct program
{
    static constexpr auto whitespace = ws_no_nl;
    static constexpr auto rule = statement_ws
                                 + dsl::opt(dsl::p<top_level_statement_list>)
                                 + statement_ws
                                 + dsl::eof;
    static constexpr auto value =
        lexy::callback<std::vector<ast::Statement::Ptr>>(
            [](lexy::nullopt) {
                return std::vector<ast::Statement::Ptr>{};
            },
            [](std::vector<ast::Statement::Ptr> stmts) {
                return stmts;
            });
    static constexpr auto name = "program";
};

} // namespace frst::grammar

#endif
