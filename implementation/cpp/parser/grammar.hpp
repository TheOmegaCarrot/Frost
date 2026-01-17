#ifndef FROST_GRAMMAR_HPP
#define FROST_GRAMMAR_HPP

#include <charconv>

#include <frost/ast.hpp>

#include <lexy/callback.hpp>
#include <lexy/dsl.hpp>
#include <lexy/dsl/ascii.hpp>
#include <lexy/lexeme.hpp>
#include <lexy/token.hpp>

#define X_KEYWORDS                                                             \
    X(if)                                                                      \
    X(else)                                                                    \
    X(elif)                                                                    \
    X(def)                                                                     \
    X(fn)                                                                      \
    X(reduce)                                                                  \
    X(map)                                                                     \
    X(foreach)                                                                 \
    X(filter)                                                                  \
    X(with)                                                                    \
    X(init)                                                                    \
    X(true)                                                                    \
    X(false)                                                                   \
    X(and)                                                                     \
    X(or)                                                                      \
    X(not)                                                                     \
    X(null)

namespace frst::grammar
{
namespace dsl = lexy::dsl;

constexpr auto ws = dsl::whitespace(dsl::ascii::space);

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

namespace node
{
struct Name_Lookup
{
    static constexpr auto rule = dsl::p<identifier>;
    static constexpr auto value =
        lexy::new_<ast::Name_Lookup, ast::Expression::Ptr>;
};
} // namespace node

} // namespace frst::grammar

#endif
