#ifndef FROST_GRAMMAR_HPP
#define FROST_GRAMMAR_HPP

#include <charconv>
#include <optional>
#include <tuple>
#include <type_traits>
#include <vector>

#include <frost/ast.hpp>
#include <frost/ast/destructure-array.hpp>
#include <frost/ast/destructure-binding.hpp>
#include <frost/ast/destructure-map.hpp>
#include <frost/ast/lambda.hpp>
#include <frost/ast/match-array.hpp>
#include <frost/ast/match-binding.hpp>
#include <frost/ast/match-map.hpp>
#include <frost/ast/match-pattern.hpp>
#include <frost/ast/match-value.hpp>
#include <frost/ast/match.hpp>
#include <frost/utils.hpp>

#include <lexy/callback.hpp>
#include <lexy/dsl.hpp>
#include <lexy/dsl/ascii.hpp>
#include <lexy/dsl/unicode.hpp>
#include <lexy/dsl/until.hpp>
#include <lexy/lexeme.hpp>
#include <lexy/token.hpp>

// =============================================================================
// HOW LEXY WORKS — read this first
// =============================================================================
//
// Lexy is a compile-time parser combinator library. Parse rules are C++
// constexpr expressions, not runtime objects. The grammar is a set of structs
// each with a `rule`, `value`, and `name` field. Lexy drives parsing by
// recursively evaluating these structs.
//
// CORE COMBINATORS:
//
//   a + b           Sequence: parse a, then b (like concatenation in EBNF).
//
//   a | b           Ordered choice: try a's *branch condition*; if it fails,
//                   try b. This is NOT backtracking — Lexy does not re-try a
//                   full parse. It only checks the branch condition (see
//                   dsl::peek below), then commits.
//
//   dsl::peek(x)    Lookahead: check whether x matches at the current position
//                   without consuming any input. Returns a *branch condition*
//                   suitable for use with `>>` or `|`.
//
//   cond >> action  Branch: if `cond` succeeds (e.g. a peek), run `action`.
//                   Once the condition passes, Lexy commits — if `action`
//                   fails, the parse fails (no backtracking to try other `|`
//                   alternatives). This is the fundamental safety mechanism:
//                   peek first to determine which branch to take, then act.
//
//   dsl::else_      A branch condition that always succeeds. Used as the final
//                   alternative in a `|` chain as a fallthrough / default case.
//
//   dsl::opt(rule)  Optional: parse `rule` if present, producing the rule's
//                   value; otherwise produce lexy::nullopt. The condition for
//                   "present" must be a branch condition (usually a peek).
//
//   dsl::list(item, sep)
//                   Parse a non-empty list of `item` with separator `sep`.
//                   `dsl::sep(s)` requires s between items.
//                   `dsl::trailing_sep(s)` also allows a trailing separator.
//
//   dsl::recurse<T> Recursive reference to production T. Required anywhere a
//                   rule refers back to itself or a mutually-recursive rule,
//                   because direct use of `dsl::p<T>` inside T's own rule
//                   would cause infinite template instantiation at compile
//                   time.
//
//   dsl::p<T>       Invoke sub-production T (non-recursive call).
//
//   dsl::token(...) Make a rule atomic: whitespace skipping is disabled inside
//                   the token. Used when internal spaces must not be allowed
//                   (e.g. a float like `3.14` must not accept `3 . 14`).
//
//   dsl::whitespace(ws)
//                   Declare automatic whitespace for a production (or globally
//                   for an expression_production). Lexy skips `ws` between
//                   every token in rules that use this whitespace setting.
//
//   dsl::while_(x)  Consume zero or more characters matching character class x.
//                   Unlike dsl::whitespace, this is explicit and manual.
//
//   dsl::must(cond).error<E>
//                   Assert that the branch condition `cond` holds right now;
//                   if it does not, emit a custom error tagged with E. Unlike
//                   peek+>>, this is not about branching — it is a hard
//                   requirement with a descriptive error message.
//
//   dsl::error<E>   Unconditionally emit error E and fail. Often used after a
//                   dsl::peek branch to provide a helpful message for a
//                   detected-but-unsupported construct (e.g. `&&` instead of
//                   `and`).
//
//   LEXY_KEYWORD(str, base)
//                   Match a keyword string AND assert an identifier boundary
//                   follows (i.e. the next char is not alphanumeric/_). This
//                   prevents `true_value` from matching the keyword `true`.
//
//   LEXY_LIT(str)   Match a literal string with no boundary check. Used for
//                   operators (`<=`, `==`) and punctuation.
//
// VALUE CALLBACKS:
//
//   lexy::callback<T>(overloads...)
//                   Construct a value of type T from the sub-values produced
//                   by the matched rule. Multiple lambda overloads handle
//                   different match shapes (e.g. with vs without optional).
//
//   lexy::forward<T>
//                   Pass-through: the production's value is whatever the
//                   sub-rule produced (used when a production is just a named
//                   alias for another rule).
//
//   lexy::as_list<V>
//                   Collect items from dsl::list into a std::vector<V>.
//
//   lexy::as_string<S>
//                   Collect a lexeme's characters into a std::string.
//
//   lexy::new_<AST_T, Ptr_T>
//                   Construct an AST node via make_unique<AST_T>. The result
//                   type is Ptr_T (usually ast::Expression::Ptr).
//
//   lexy::nullopt   The type produced by dsl::opt when no match occurs.
//                   Overloads in lexy::callback use it as a discriminant to
//                   handle the "absent" case.
//
// ERROR TAGS:
//
//   Error tags are empty structs with a `static constexpr auto name = "..."`.
//   Lexy extracts the `name` field for its error messages. They are passed as
//   template arguments to dsl::error<E>, dsl::must(...).error<E>, etc.
//
// EXPRESSION PRODUCTIONS:
//
//   lexy::expression_production is a mixin that implements the operator
//   precedence tower. Subclasses declare:
//     - `atom`: the lowest-level (non-operator) expression.
//     - `whitespace`: automatic whitespace between tokens.
//     - Operator structs (each inheriting from dsl::prefix_op,
//       dsl::postfix_op, dsl::infix_op_left, or dsl::infix_op_single) with:
//       - `op`: the operator token(s), defined with dsl::op<tag>(rule).
//       - `using operand = X`: the next-lower precedence level.
//     - `using operation = X`: the top-level precedence level.
//   The `/` operator on operator lists means "alternative operator token",
//   analogous to `|` for productions.
//
// WHITESPACE / NEWLINES:
//
//   Frost uses newlines as statement terminators. The grammar therefore has two
//   modes: "allow newlines" (inside delimiters where newlines are harmless) and
//   "no newlines" (between tokens at statement level). Many rules come in
//   `_impl<allow_nl>` template pairs, instantiated as separate aliases.
//   `ws_no_nl` and `ws_nl` are the two whitespace definitions; `param_ws` and
//   `param_ws_nl` are the manual equivalents for contexts that use dsl::while_.
//
// =============================================================================

namespace frst::grammar
{
namespace dsl = lexy::dsl;

// =============================================================================
// Source position infrastructure
// =============================================================================
//
// `dsl::position` captures a raw iterator (e.g. `const char8_t*`) at the
// current reader position.  To convert this to a human-readable {line,column}
// we need the start of the input buffer.  A thread_local stores this pointer;
// `reset_parse_state()` must be called before every `lexy::parse` invocation.
//
// An anchor cache avoids re-scanning from the beginning on every call.
// Because parsing proceeds left-to-right the amortised cost is O(n) total.

namespace detail
{
inline thread_local const char8_t* g_input_begin = nullptr;
inline thread_local const char8_t* g_anchor_pos = nullptr;
inline thread_local std::size_t g_anchor_line = 1;
inline thread_local std::size_t g_anchor_col = 1;
} // namespace detail

template <typename Input>
void reset_parse_state(const Input& input)
{
    auto begin = input.data();
    detail::g_input_begin = reinterpret_cast<const char8_t*>(begin);
    detail::g_anchor_pos = detail::g_input_begin;
    detail::g_anchor_line = 1;
    detail::g_anchor_col = 1;
}

inline ast::AST_Node::Source_Location to_source_location(const auto& pos)
{
    auto p = reinterpret_cast<const char8_t*>(&*pos);

    const char8_t* scan_from;
    std::size_t line, col;

    if (detail::g_anchor_pos && detail::g_anchor_pos <= p)
    {
        scan_from = detail::g_anchor_pos;
        line = detail::g_anchor_line;
        col = detail::g_anchor_col;
    }
    else
    {
        scan_from = detail::g_input_begin;
        line = 1;
        col = 1;
    }

    for (auto c = scan_from; c < p; ++c)
    {
        if (*c == static_cast<char8_t>('\n'))
        {
            ++line;
            col = 1;
        }
        else
        {
            ++col;
        }
    }

    detail::g_anchor_pos = p;
    detail::g_anchor_line = line;
    detail::g_anchor_col = col;

    return {.line = line, .column = col};
}

// Scan backward from `end` past automatically-consumed whitespace and
// line comments.  Lexy's auto-whitespace inserts invisible gaps between
// a token's last character and the `dsl::position` that follows; this
// function finds the exclusive-end of the actual content.
//
// `stop` is a lower bound — scanning will never go below it.
inline const char8_t* skip_trailing_ws(const char8_t* stop, const char8_t* end)
{
    while (end > stop)
    {
        auto c = *(end - 1);
        if (c
            == u8' '
            || c
            == u8'\t'
            || c
            == u8'\n'
            || c
            == u8'\r'
            || c
            == u8'\f'
            || c
            == u8'\v')
        {
            --end;
            continue;
        }
        // Check for a line comment (`# ... \n`).  Scan backward on
        // the current line looking for `#`.
        auto scan = end;
        bool found_hash = false;
        while (scan > stop)
        {
            --scan;
            if (*scan == u8'\n')
                break;
            if (*scan == u8'#')
            {
                found_hash = true;
                break;
            }
        }
        if (found_hash)
        {
            end = scan;
            continue;
        }
        break;
    }
    return end;
}

inline ast::AST_Node::Source_Range make_source_range(const auto& begin_pos,
                                                     const auto& end_pos)
{
    auto begin_p = reinterpret_cast<const char8_t*>(&*begin_pos);
    auto end_p = reinterpret_cast<const char8_t*>(&*end_pos);
    if (end_p <= begin_p)
        return ast::AST_Node::no_range;
    // Strip trailing whitespace/comments that Lexy consumed after the
    // last token to find the true content end.
    auto actual_end = skip_trailing_ws(begin_p, end_p);
    if (actual_end <= begin_p)
        return ast::AST_Node::no_range;
    auto begin_loc = to_source_location(begin_p);
    auto end_loc = to_source_location(actual_end - 1);
    return {.begin = begin_loc, .end = end_loc};
}

// Compute inclusive end location from a one-past-end position.
inline ast::AST_Node::Source_Location inclusive_end_loc(
    const auto& one_past_end_pos)
{
    auto end_p = reinterpret_cast<const char8_t*>(&*one_past_end_pos);
    auto actual_end = skip_trailing_ws(detail::g_input_begin, end_p);
    return to_source_location(actual_end - 1);
}

// Compute the source location of a prefix operator's first character.
// `after_ws_pos` is the position captured AFTER the operator token and any
// automatically-consumed whitespace.  We scan backward past whitespace to
// find the last character of the operator, then step back `op_len - 1` to
// reach its first character.
inline ast::AST_Node::Source_Location prefix_op_begin(const auto& after_ws_pos,
                                                      std::size_t op_len)
{
    auto p = reinterpret_cast<const char8_t*>(&*after_ws_pos);
    --p;
    while (*p == u8' ' || *p == u8'\t' || *p == u8'\n' || *p == u8'\r')
        --p;
    // p now points to the last character of the operator token.
    p -= (op_len - 1);
    return to_source_location(p);
}

// =============================================================================
// Whitespace definitions
// =============================================================================
//
// Frost has two whitespace contexts:
//
//   ws_no_nl  — horizontal whitespace only (spaces, tabs, CR, FF, VT) plus
//               line comments. Used at statement level where a newline
//               terminates a statement.
//
//   ws_nl     — all ASCII whitespace including newlines, plus line comments.
//               Used inside delimiters (arrays, maps, call arg lists, etc.)
//               where newlines are just formatting.
//
// `dsl::whitespace(x)` registers x as automatic whitespace for an expression
// production: Lexy silently skips it between every token in that production.
//
// `param_ws` / `param_ws_nl` are the manual equivalents using `dsl::while_`.
// They are used in places where we need explicit control (e.g. inside
// separators and list delimiters) rather than relying on automatic skipping.
//
// `statement_ws` also consumes semicolons and newlines — it is the separator
// between statements in a block.

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

// =============================================================================
// Comma separators
// =============================================================================
//
// Commas in Frost are surrounded by optional whitespace. These helpers
// produce branch conditions (using dsl::peek) so they can be used as
// `dsl::sep(...)` or `dsl::trailing_sep(...)` arguments: Lexy checks the
// peek before committing to consuming the comma.
//
// `comma_sep_after` and `comma_sep_nl_after` are "context-aware" variants that
// also peek at what *follows* the comma. This is used in separator contexts
// where the next item has a known start token — it prevents the parser from
// consuming a comma that was meant to be followed by something else (e.g., the
// closing delimiter).

template <typename ws_t>
constexpr auto comma_sep_ws(ws_t ws)
{
    return dsl::peek(ws + dsl::lit_c<','>) >> (ws + dsl::lit_c<','> + ws);
}

constexpr auto comma_sep_nl = comma_sep_ws(param_ws_nl);
constexpr auto comma_sep = comma_sep_ws(param_ws);

template <typename after_t>
constexpr auto comma_sep_after(after_t after)
{
    return dsl::peek(param_ws + dsl::lit_c<','> + param_ws + after)
           >> (param_ws + dsl::lit_c<','> + param_ws);
}

template <typename after_t>
constexpr auto comma_sep_nl_after(after_t after)
{
    return dsl::peek(param_ws_nl + dsl::lit_c<','> + param_ws_nl + after)
           >> (param_ws_nl + dsl::lit_c<','> + param_ws_nl);
}

// =============================================================================
// Expression start detection
// =============================================================================
//
// Many rules need to know whether an expression follows, without consuming
// anything. `expression_start_impl<allow_nl>()` returns a peek condition that
// checks whether the next non-whitespace character is a valid expression
// starter.
//
// This is used in two ways:
//
//   1. As a branch condition (with `>>`): enter an expression-parsing branch
//      only if we know one is coming, so that a missing expression produces a
//      helpful error rather than silently producing an empty list or the like.
//
//   2. In dsl::must(...).error<E> (via require_expr_start_*): assert that an
//      expression is required here, and produce a named error if it is absent.
//
// The `no_nl` variant skips only horizontal whitespace before checking, so a
// leading newline does NOT count as an expression start. The `nl` variant
// skips all whitespace including newlines. This distinction is what prevents
// a newline after a statement from being misread as the start of a
// continuation.

template <bool allow_nl>
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
    if constexpr (allow_nl)
        return dsl::peek(param_ws_nl + starter);
    else
        return dsl::peek(param_ws + starter);
}

constexpr auto expression_start_no_nl = expression_start_impl<false>();
constexpr auto expression_start_nl = expression_start_impl<true>();
constexpr auto expression_start = expression_start_no_nl;

// =============================================================================
// Error tag structs
// =============================================================================
//
// These empty structs serve as error type tags. Lexy reads the `name` field
// to produce the user-facing error text. They are passed as template arguments
// to dsl::error<E> (unconditional error emission) or dsl::must(...).error<E>
// (conditional error assertion).
//
// The naming convention is:
//   expected_*   — "I expected to see X here but it was missing"
//   use_*        — "this construct is wrong; use Y instead"
//   unexpected_* — "this token appeared without its required context"

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
    static constexpr auto name = "map entry value";
};

struct expected_init_expression
{
    static constexpr auto name = "expression after 'init:'";
};

struct expected_call_arguments
{
    static constexpr auto name = "closing ')'";
};

// Helpers that combine dsl::must with a specific expected_expression error.
// `require_expr_start_no_nl()` asserts that an expression starts here
// (no newline allowed before it); `require_expr_start_nl()` allows newlines.
// These are used after a keyword or colon where an expression is mandatory,
// so that "missing expression" produces a clear error rather than a confusing
// one about the next unrelated token.
template <typename expected_t>
constexpr auto require_expr_start_no_nl()
{
    return dsl::must(expression_start_no_nl).template error<expected_t>;
}

template <typename expected_t>
constexpr auto require_expr_start_nl()
{
    return dsl::must(expression_start_nl).template error<expected_t>;
}

struct expected_index_expression
{
    static constexpr auto name = "closing ']'";
};

// "Helpful mistake" errors: the parser detects a common error pattern and
// emits a message that tells the user exactly what to do instead.
struct use_elif_not_else_if
{
    static constexpr auto name = "use 'elif' instead of 'else if'";
};

struct use_and_not_double_ampersand
{
    static constexpr auto name = "use 'and' instead of '&&'";
};

struct use_or_not_double_pipe
{
    static constexpr auto name = "use 'or' instead of '||'";
};

struct use_def_for_assignment
{
    static constexpr auto name = "use 'def name = value' to define a variable";
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

struct expected_match_pattern
{
    static constexpr auto name = "match pattern";
};

struct expected_type_constraint
{
    static constexpr auto name = "type constraint name (after `is`)";
};

struct expected_match_arrow
{
    static constexpr auto name = "`=>` after match pattern (or guard)";
};

struct expected_match_arm_result
{
    static constexpr auto name = "expression after `=>`";
};

struct expected_match_guard_expression
{
    static constexpr auto name = "expression after `if:`";
};

struct expected_match_target
{
    static constexpr auto name = "expression after `match`";
};

// =============================================================================
// Identifiers
// =============================================================================
//
// `identifier` matches any valid Frost identifier that is NOT a reserved
// keyword. The reserved keyword check is done by
// `identifier::base.reserve(...)`, which takes the keyword list from the
// FROST_X_KEYWORDS macro. If the matched text is a keyword, the identifier rule
// fails (and the keyword rule will succeed instead).
//
// LEXY_KEYWORD(str, base) requires that after the literal `str`, the next
// character is NOT an identifier continuation character (letter/digit/_). This
// prevents `null_value` from matching `null` as a keyword. The `base`
// argument tells Lexy what character set defines "identifier boundary".
//
// `identifier_required` wraps `identifier` with a `dsl::must` guard that fires
// `expected_identifier` if the current character is not even an identifier
// start. This is used in positions where an identifier is grammatically
// required (map keys, parameter names, etc.).

struct identifier
{
    static constexpr auto base =
        dsl::identifier(dsl::ascii::alpha_underscore, dsl::ascii::word);

    static constexpr auto reserved = [] {
#define X(keyword) LEXY_KEYWORD(keyword, base),
        constexpr auto keyword_list = std::tuple{FROST_X_KEYWORDS};
#undef X

        return std::apply(
            [](const auto&... keywords) {
                return base.reserve(keywords...);
            },
            keyword_list);
    }();

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

// =============================================================================
// Literals
// =============================================================================

// Integer: Lexy's built-in dsl::integer<T> handles decimal digits and
// produces a value of type T.
struct integer_literal
{
    static constexpr auto rule = dsl::integer<Int>;
    static constexpr auto value = lexy::callback<Value_Ptr>([](Int value) {
        return Value::create(auto{value});
    });
    static constexpr auto name = "integer literal";
};

// Shared logic: parse a captured float lexeme string via from_chars.
template <typename Iter>
inline Value_Ptr do_parse_float(Iter begin, Iter end)
{
    std::string str{begin, end};
    Float value{};
    auto [ptr, ec] = std::from_chars(str.data(), str.data() + str.size(),
                                     value, std::chars_format::general);
    if (ec == std::errc::result_out_of_range)
        throw Frost_Recoverable_Error{"Float literal out of range"};
    if (ec != std::errc() || ptr != str.data() + str.size())
        throw Frost_Interpreter_Error{"Failed to parse float literal"};
    return Value::create(auto{value});
}

// Float literal: digits . digits [e [+-] digits]
// After capturing the decimal form atomically via dsl::token, the scanner
// greedily consumes a trailing exponent by walking the source pointer.
// This avoids whitespace pollution from scanner.position() and sidesteps
// lexy's scan_production backtracking limitations.
struct float_literal : lexy::scan_production<Value_Ptr>
{
    template <typename context_t, typename reader_t>
    static constexpr scan_result scan(
        lexy::rule_scanner<context_t, reader_t>& scanner)
    {
        auto lexeme = scanner.capture(dsl::token(
            dsl::digits<> + dsl::lit_c<'.'> + dsl::digits<>
            + dsl::opt(dsl::lit_c<'e'>
                       >> dsl::opt(dsl::lit_c<'+'> / dsl::lit_c<'-'>)
                       + dsl::digits<>)));
        if (not lexeme)
            return scan_result{};

        auto text = lexeme.value();
        return do_parse_float(text.begin(), text.end());
    }
    static constexpr auto name = "float literal";
};

// Scientific float without decimal: digits e [+-] digits (e.g. 1e+20)
// Scientific float without decimal point: digits e [+-]? digits (e.g. 1e+20, 3e2)
struct scientific_float_literal : lexy::scan_production<Value_Ptr>
{
    template <typename context_t, typename reader_t>
    static constexpr scan_result scan(
        lexy::rule_scanner<context_t, reader_t>& scanner)
    {
        auto lexeme = scanner.capture(dsl::token(
            dsl::digits<>
            + dsl::lit_c<'e'>
            + dsl::opt(dsl::lit_c<'+'> / dsl::lit_c<'-'>)
            + dsl::digits<>));
        if (not lexeme)
            return scan_result{};
        auto text = lexeme.value();
        return do_parse_float(text.begin(), text.end());
    }
    static constexpr auto name = "scientific float literal";
};

// Boolean literals: LEXY_KEYWORD is essential here — without it, an identifier
// like `true_value` would match `true` as a prefix, leaving `_value` as a
// confusing leftover. The keyword macro checks that the keyword is not followed
// by an identifier character.
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

// String literals: four inner sub-types tried in order.
//
// raw_r_double / raw_r_single: R"(...)" and R'(...)' raw strings. The limit
//   on dsl::ascii::newline means raw strings cannot span multiple lines.
//   No escape sequences — the content is taken verbatim.
//
// raw_double / raw_single: regular "..." and '...' strings. Also single-line
//   (limit on newline). Support backslash escapes defined in the symbol table.
//   The limit is what makes the token atomic in the newline sense — if a
//   newline is encountered before the closing delimiter, the rule fails rather
//   than consuming the newline as content.
//
// The ordering matters: raw_r_double and raw_r_single are tried first because
// they start with `R"` / `R'`, which are longer prefixes that must be checked
// before the plain `"` / `'` alternatives.
struct string_literal
{
    // Escape sequences shared by double-quoted and single-quoted strings.
    // Each variant extends this with its own delimiter escape.
    static constexpr auto common_escapes = lexy::symbol_table<char>
                                               .map<'n'>('\n')
                                               .map<'t'>('\t')
                                               .map<'r'>('\r')
                                               .map<'\\'>('\\')
                                               .map<'0'>('\0');

    // \xNN: two hex digits interpreted as a raw byte.
    static constexpr auto hex_escape =
        dsl::lit_c<'x'> >> dsl::code_unit_id<lexy::byte_encoding, 2>;

    struct raw_r_double
    {
        static constexpr auto rule =
            dsl::delimited(LEXY_LIT("R\"("), LEXY_LIT(")\""))
                .limit(dsl::ascii::newline)(dsl::unicode::character);
        static constexpr auto value = lexy::as_string<std::string>;
        static constexpr auto name = "string literal";
    };

    struct raw_r_single
    {
        static constexpr auto rule =
            dsl::delimited(LEXY_LIT("R'("), LEXY_LIT(")'"))
                .limit(dsl::ascii::newline)(dsl::unicode::character);
        static constexpr auto value = lexy::as_string<std::string>;
        static constexpr auto name = "string literal";
    };

    struct raw_double
    {
        static constexpr auto escapes = common_escapes.map<'"'>('"');

        static constexpr auto rule = dsl::quoted.limit(dsl::ascii::newline)(
            dsl::unicode::character,
            dsl::backslash_escape.symbol<escapes>().rule(hex_escape));
        static constexpr auto value = lexy::as_string<std::string>;
        static constexpr auto name = "string literal";
    };

    struct raw_single
    {
        static constexpr auto escapes = common_escapes.map<'\''>('\'');

        static constexpr auto rule =
            dsl::single_quoted.limit(dsl::ascii::newline)(
                dsl::unicode::character,
                dsl::backslash_escape.symbol<escapes>().rule(hex_escape));
        static constexpr auto value = lexy::as_string<std::string>;
        static constexpr auto name = "string literal";
    };

    // Tag type to distinguish triple-quoted content for indentation trimming.
    struct Multiline_Content
    {
        std::string text;
    };

    // Escape sequences for triple-quoted strings: \n, \r, and \xNN are
    // forbidden because actual newlines are available and escape-produced
    // newlines interact poorly with indentation trimming.
    static constexpr auto multiline_escapes =
        lexy::symbol_table<char>.map<'t'>('\t').map<'\\'>('\\').map<'0'>('\0');

    // Triple-quoted multiline strings: """...""" and '''...'''
    // Escape sequences are processed. Indentation is trimmed based
    // on the closing delimiter's position.
    struct triple_double_inner
    {
        static constexpr auto escapes = multiline_escapes.map<'"'>('"');

        static constexpr auto rule = dsl::delimited(LEXY_LIT("\"\"\""),
                                                    LEXY_LIT("\"\"\""))(
            dsl::unicode::character, dsl::backslash_escape.symbol<escapes>());
        static constexpr auto value = lexy::as_string<std::string>;
        static constexpr auto name = "string literal";
    };

    struct triple_double
    {
        static constexpr auto rule = dsl::p<triple_double_inner>;
        static constexpr auto value =
            lexy::callback<Multiline_Content>([](std::string s) {
                return Multiline_Content{std::move(s)};
            });
    };

    struct triple_single_inner
    {
        static constexpr auto escapes = multiline_escapes.map<'\''>('\'');

        static constexpr auto rule = dsl::delimited(LEXY_LIT("'''"),
                                                    LEXY_LIT("'''"))(
            dsl::unicode::character, dsl::backslash_escape.symbol<escapes>());
        static constexpr auto value = lexy::as_string<std::string>;
        static constexpr auto name = "string literal";
    };

    struct triple_single
    {
        static constexpr auto rule = dsl::p<triple_single_inner>;
        static constexpr auto value =
            lexy::callback<Multiline_Content>([](std::string s) {
                return Multiline_Content{std::move(s)};
            });
    };

    static constexpr auto rule = dsl::p<raw_r_double>
                                 | dsl::p<raw_r_single>
                                 | dsl::p<triple_double>
                                 | dsl::p<triple_single>
                                 | dsl::p<raw_double>
                                 | dsl::p<raw_single>;
    static constexpr auto value = lexy::callback<Value_Ptr>(
        [](std::string str) {
            return Value::create(std::move(str));
        },
        [](Multiline_Content content) {
            auto result = utils::trim_multiline_indentation(content.text);
            if (not result.has_value())
                throw Frost_Interpreter_Error{result.error()};
            return Value::create(std::move(result).value());
        });
    static constexpr auto name = "string literal";
};

// Format strings reuse the `raw_double`/`raw_single` string rules but not the
// raw R"(...)" variants. dsl::no_whitespace (in node::Format_String) ensures
// the `$` sigil and opening quote are adjacent.
struct format_string_literal
{
    static constexpr auto rule =
        dsl::p<string_literal::raw_double> | dsl::p<string_literal::raw_single>;
    static constexpr auto value = lexy::forward<std::string>;
    static constexpr auto name = "format string literal";
};

// =============================================================================
// AST-wrapping nodes (namespace node::)
// =============================================================================
//
// The `node::` namespace contains thin wrappers that take the raw values
// produced by the grammar rules above and construct AST nodes from them.
// This separation keeps the grammar logic (how to parse) distinct from the
// AST construction logic (what to build).

namespace node
{
// Literal: tries null, bool, string, float, then integer. The float/integer
// ordering is important: float is tried first via a peek at the pattern
// `digits<> + '.' + digit<>` (singular digit<> after the dot — requires at
// least one digit, not zero), so `3.14` is a float and `3` is an integer.
// Both peeks prevent committing to a parse that will fail.
struct Literal
{
    static constexpr auto rule =
        dsl::p<null_literal>
        | dsl::p<bool_literal>
        | dsl::p<string_literal>
        | (dsl::peek(dsl::token(dsl::digits<> + dsl::lit_c<'.'> + dsl::digit<>))
           >> dsl::p<float_literal>)
        | (dsl::peek(dsl::token(
               dsl::digits<> + dsl::lit_c<'e'>
               + (dsl::lit_c<'+'> / dsl::lit_c<'-'>)))
           >> dsl::p<scientific_float_literal>)
        | (dsl::peek(dsl::token(
               dsl::digits<> + dsl::lit_c<'e'> + dsl::digit<>))
           >> dsl::p<scientific_float_literal>)
        | (dsl::peek(dsl::digit<>) >> dsl::p<integer_literal>);
    static constexpr auto value =
        lexy::callback<ast::Expression::Ptr>([](Value_Ptr val) {
            return std::make_unique<ast::Literal>(ast::AST_Node::no_range,
                                                  std::move(val));
        });
    static constexpr auto name = "literal";
};

// Format string: `$"..."`. dsl::no_whitespace disables automatic whitespace
// skipping so that `$ "foo"` (with a space) is rejected — the `$` and the
// opening quote must be adjacent.
struct Format_String
{
    static constexpr auto rule =
        dsl::no_whitespace(dsl::lit_c<'$'> >> dsl::p<format_string_literal>);
    static constexpr auto value =
        lexy::callback<ast::Expression::Ptr>([](std::string format) {
            return std::make_unique<ast::Format_String>(ast::AST_Node::no_range,
                                                        std::move(format));
        });
    static constexpr auto name = "format string";
};

// Name lookup: a bare identifier used as an expression. The `identifier` rule
// already rejects keywords, so this will never accidentally match `if`, `def`,
// etc.
struct Name_Lookup
{
    static constexpr auto rule = dsl::p<identifier>;
    static constexpr auto value =
        lexy::callback<ast::Expression::Ptr>([](std::string name) {
            return std::make_unique<ast::Name_Lookup>(ast::AST_Node::no_range,
                                                      std::move(name));
        });
    static constexpr auto name = "name";
};
} // namespace node

// =============================================================================
// Forward declarations (required by recursive rules)
// =============================================================================
//
// `expression` and `expression_nl` appear in many sub-rules. Since C++
// requires declarations before use and the grammar is mutually recursive,
// we forward-declare them here. The actual definitions appear later.
// `statement_list` is forward-declared for the same reason (lambda bodies
// reference it).

struct expression;
struct expression_nl;
struct statement_list;

// =============================================================================
// List helpers
// =============================================================================
//
// These templates factor out common patterns for parsing delimited lists.

// `list_or_empty<T>`: combines lexy::as_list (which collects items into a
// vector) with a nullopt handler. When dsl::opt produces lexy::nullopt (the
// list was absent), we return an empty vector. This simplifies callers that
// use dsl::opt(... >> list).
template <typename item_t>
constexpr auto list_or_empty()
{
    return lexy::as_list<std::vector<item_t>> >> lexy::
               callback<std::vector<item_t>>(
                   [](lexy::nullopt) {
                       return std::vector<item_t>{};
                   },
                   [](std::vector<item_t> items) {
                       return items;
                   });
}

// `delimited_list_tail`: parse the contents and closing delimiter of an
// already-opened delimited list. The `entry_start` peek guards the list so
// an empty delimited region produces an empty vector rather than an error.
template <typename entry_start_t, typename list_t, typename close_t>
constexpr auto delimited_list_tail(entry_start_t entry_start, list_t list,
                                   close_t close)
{
    return param_ws_nl
           + dsl::opt(dsl::peek(entry_start) >> list)
           + param_ws_nl
           + close;
}

// `delimited_list`: add the opening delimiter before `delimited_list_tail`.
template <typename open_t, typename entry_start_t, typename list_t,
          typename close_t>
constexpr auto delimited_list(open_t open, entry_start_t entry_start,
                              list_t list, close_t close)
{
    return open + delimited_list_tail(entry_start, list, close);
}

// `braced_list`: specialization of `delimited_list` for `{...}`.
template <typename entry_start_t, typename list_t>
constexpr auto braced_list(entry_start_t entry_start, list_t list)
{
    return delimited_list(LEXY_LIT("{"), entry_start, list, dsl::lit_c<'}'>);
}

// =============================================================================
// Postfix operator helpers
// =============================================================================
//
// Factored out so both the main expression production and the inner `callee`
// sub-production of threaded calls can share the same index/dot postfix logic.

// Index: `expr[index]`. After `[`, we require an expression start (with a
// named error); if that passes, we recurse into expression_nl (allowing
// newlines inside `[...]`) and then expect `]`.
template <typename op_index_t>
constexpr auto index_postfix_op()
{
    return dsl::op<op_index_t>(
        dsl::
            lit_c<'['> >> (param_ws_nl
                           + (require_expr_start_nl<expected_index_expression>()
                              >> dsl::recurse<expression_nl>)+param_ws_nl
                           + dsl::lit_c<']'>
                           + dsl::position));
}

// Dot: `expr.field`. After `.`, an identifier is required. The identifier
// is converted to a string-key index at AST construction time.
// Two positions are captured: one before the identifier (for the key range
// begin) and one after (for the key range end / index expression end).
template <typename op_dot_t>
constexpr auto dot_postfix_op()
{
    return dsl::op<op_dot_t>(dsl::lit_c<'.'> >> (dsl::position
                                                 + dsl::p<identifier_required>
                                                 + dsl::position));
}

// =============================================================================
// Lambda / destructure parameter structures
// =============================================================================

// Plain parameter list (no varargs). The comma separator peeks ahead to ensure
// that the next item after the comma is an identifier start — this avoids
// consuming a comma that is followed by `)` or `...`.
struct lambda_param_pack
{
    std::vector<std::string> params;
    std::optional<std::string> vararg;
    std::optional<std::string> self_name;
};

struct destructure_pack
{
    std::vector<ast::Destructure::Ptr> elements;
    std::optional<std::string> rest_name;
};

// Forward declarations for recursive destructure patterns.
template <bool exported>
struct array_destructure_pattern_impl;
template <bool exported>
struct map_destructure_entries_impl;

// A single destructure pattern element. Recursively allows nested
// array/map destructuring, or an identifier binding.
template <bool exported>
struct destructure_pattern_impl
{
    static constexpr auto rule =
        dsl::peek(dsl::lit_c<'['>)
        >> (dsl::position
            + dsl::recurse<array_destructure_pattern_impl<exported>>
            + dsl::position)
        | dsl::peek(dsl::lit_c<'{'>)
        >> (dsl::position
            + dsl::recurse<map_destructure_entries_impl<exported>>
            + dsl::position)
        | dsl::else_
        >> (dsl::position + dsl::p<identifier_required> + dsl::position);

    static constexpr auto value = lexy::callback<ast::Destructure::Ptr>(
        // From array destructure
        [](auto begin, destructure_pack pack, auto end) {
            return std::make_unique<ast::Destructure_Array>(
                make_source_range(begin, end), std::move(pack.elements),
                std::move(pack.rest_name), exported);
        },
        // From map destructure
        [](auto begin, std::vector<ast::Destructure_Map::Element> elems,
           auto end) {
            return std::make_unique<ast::Destructure_Map>(
                make_source_range(begin, end), std::move(elems));
        },
        // Leaf identifier
        [](auto begin, std::string name, auto end) -> ast::Destructure::Ptr {
            std::optional<std::string> opt_name =
                name == "_" ? std::nullopt
                            : std::optional<std::string>{std::move(name)};
            return std::make_unique<ast::Destructure_Binding>(
                make_source_range(begin, end), std::move(opt_name), exported);
        });
    static constexpr auto name = "destructure pattern";
};

template <bool exported>
struct destructure_pattern_list_impl
{
    static constexpr auto rule = [] {
        auto elem_start =
            dsl::ascii::alpha_underscore / dsl::lit_c<'['> / dsl::lit_c<'{'>;
        auto item = dsl::peek(elem_start)
                    >> dsl::recurse<destructure_pattern_impl<exported>>;
        // Trailing comma is allowed: `[a, b,]`. Broaden the separator's
        // peek to also accept the closing `]` so the list commits to the
        // comma even when no further element follows.
        auto sep = comma_sep_nl_after(elem_start | dsl::lit_c<']'>);
        return dsl::list(item, dsl::trailing_sep(sep));
    }();
    static constexpr auto value =
        lexy::as_list<std::vector<ast::Destructure::Ptr>>;
    static constexpr auto name = "destructure patterns";
};

// Rest binding in array destructure: `...identifier` only (not a pattern).
struct destructure_rest_binding
{
    static constexpr auto rule = dsl::p<identifier_required>;
    static constexpr auto value =
        lexy::callback<std::string>([](std::string name) {
            return name;
        });
    static constexpr auto name = "rest binding";
};

// `destructure_payload`: the contents of a `[...]` destructure pattern.
//
// Valid forms:
//   []             — empty: binds nothing
//   [a, b]         — pattern bindings only
//   [...rest]      — rest binding only
//   [a, b, ...rest] — pattern bindings followed by a rest binding
template <bool exported>
struct destructure_payload_impl
{
    static constexpr auto rule = [] {
        auto leading_comma = dsl::peek(dsl::lit_c<','>)
                             >> dsl::error<expected_destructure_binding>;
        auto rest = (dsl::must(dsl::peek(LEXY_LIT("...")))
                         .template error<
                             expected_destructure_rest
                         > >> (LEXY_LIT("...")
                               + param_ws_nl
                               + dsl::p<destructure_rest_binding>));
        auto rest_checked = rest
                            + dsl::must(dsl::peek_not(dsl::lit_c<','>))
                                  .template error<expected_vararg_last>;
        auto patterns = dsl::p<destructure_pattern_list_impl<exported>>;
        auto elem_start =
            dsl::ascii::alpha_underscore / dsl::lit_c<'['> / dsl::lit_c<'{'>;
        auto patterns_then_rest =
            patterns
            + dsl::opt(dsl::peek(dsl::lit_c<','>)
                       >> (dsl::lit_c<','> + param_ws_nl + rest_checked));
        auto rest_only = dsl::peek(LEXY_LIT("...")) >> rest_checked;
        auto patterns_only = dsl::peek(elem_start) >> patterns_then_rest;
        return dsl::opt(rest_only | patterns_only | leading_comma);
    }();

    static constexpr auto value = lexy::callback<destructure_pack>(
        [](lexy::nullopt) {
            return destructure_pack{};
        },
        [](std::string rest) {
            return destructure_pack{{}, std::move(rest)};
        },
        [](std::vector<ast::Destructure::Ptr> elems, lexy::nullopt) {
            return destructure_pack{std::move(elems), std::nullopt};
        },
        [](std::vector<ast::Destructure::Ptr> elems, std::string rest) {
            return destructure_pack{std::move(elems), std::move(rest)};
        });
    static constexpr auto name = "destructure payload";
};

template <bool exported>
struct array_destructure_pattern_impl
{
    static constexpr auto rule = dsl::lit_c<'['>
                                 + param_ws_nl
                                 + dsl::p<destructure_payload_impl<exported>>
                                 + param_ws_nl
                                 + dsl::lit_c<']'>;
    static constexpr auto value = lexy::forward<destructure_pack>;
    static constexpr auto name = "array destructure";
};

// =============================================================================
// Lambda parameter lists
// =============================================================================
//
// Lambda parameters exist in two syntactic positions:
//
//   Parenthesized: `fn (a, b, ...rest) -> body`
//   Elided parens: `fn a, b -> body` or `fn x -> body`
//
// The `allow_nl` template parameter controls whether newlines are permitted
// between parameters (allowed inside explicit `(...)`, not allowed in the
// elided form where a newline would end the statement).
//
// `lambda_param_list_impl` parses the parameter names separated by commas.
// `lambda_param_payload_impl` wraps it to also handle the optional `...name`
// vararg at the end, plus the edge cases of vararg-only or empty param lists.
//
// The vararg must come last; `expected_vararg_last` fires if a comma follows
// the `...name`. This is checked via dsl::must(dsl::peek_not(',')).

template <bool allow_nl>
struct lambda_param_list_impl
{
    static constexpr auto rule = [] {
        if constexpr (allow_nl)
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

template <bool allow_nl>
struct lambda_param_payload_impl
{
    static constexpr auto rule = [] {
        auto vararg_name = [] {
            if constexpr (allow_nl)
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
        auto params = dsl::p<lambda_param_list_impl<allow_nl>>;
        auto params_then_vararg = [&] {
            if constexpr (allow_nl)
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
            return lambda_param_pack{{}, std::move(vararg), std::nullopt};
        },
        [](std::vector<std::string> params, lexy::nullopt) {
            return lambda_param_pack{std::move(params), std::nullopt,
                                     std::nullopt};
        },
        [](std::vector<std::string> params, std::string vararg) {
            return lambda_param_pack{std::move(params), std::move(vararg),
                                     std::nullopt};
        });
    static constexpr auto name = "lambda parameters";
};

using lambda_param_payload = lambda_param_payload_impl<false>;
using lambda_param_payload_nl = lambda_param_payload_impl<true>;

// `lambda_parameters_paren`: `(a, b, ...rest)` — parenthesized form.
// Newlines are allowed inside the parens.
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

// `lambda_parameters_elided`: `a, b` or `a` — no parentheses.
// Newlines are NOT allowed here, because `fn x\n+ 1` would otherwise
// parse as `fn x -> (+ 1)` rather than `(fn x -> ...) + 1`.
struct lambda_parameters_elided
{
    static constexpr auto rule = dsl::p<lambda_param_payload>;
    static constexpr auto value = lexy::forward<lambda_param_pack>;
    static constexpr auto name = "lambda parameters";
};

// `lambda_param_clause`: the parameter clause before `->`.
// Three forms (in priority order):
//   Named:       `fn f(a, b) -> body`   — identifier then `(` (no newline
//   between) Parenthesized: `fn (a, b) -> body`  — `(` directly (newlines ok
//   before it) Elided:      `fn a, b -> body`      — no parens
struct lambda_param_clause
{
    static constexpr auto rule = [] {
        auto kw_arrow = LEXY_LIT("->");

        // Named branch: identifier immediately followed by `(` (no newline).
        auto named_peek = dsl::peek(param_ws
                                    + dsl::ascii::alpha_underscore
                                    + dsl::while_(dsl::ascii::word)
                                    + param_ws
                                    + dsl::lit_c<'('>);
        auto named_branch = param_ws
                            + dsl::p<identifier_required>
                            + param_ws
                            + dsl::p<lambda_parameters_paren>;

        auto params = named_peek
                      >> named_branch
                      | dsl::peek(param_ws_nl + dsl::lit_c<'('>)
                      >> (param_ws_nl + dsl::p<lambda_parameters_paren>)
                      | dsl::else_
                      >> (param_ws_nl + dsl::p<lambda_parameters_elided>);

        return params + param_ws_nl + kw_arrow;
    }();

    static constexpr auto value = lexy::callback<lambda_param_pack>(
        [](std::string name, lambda_param_pack inner) -> lambda_param_pack {
            inner.self_name = std::move(name);
            return inner;
        },
        [](lambda_param_pack pack) -> lambda_param_pack {
            return pack;
        });
    static constexpr auto name = "lambda parameters";
};

// =============================================================================
// Lambda body
// =============================================================================
//
// The lambda body follows `->` and is either an expression or a block:
//
//   fn x -> x + 1          — expression body
//   fn x -> { def y = x    — block body (curly braces, one or more statements)
//              y + 1 }
//
// The ambiguity: `{` can start either a map literal (expression body) or a
// block (statement body). The parser must distinguish them with bounded
// lookahead because Lexy is non-backtracking.
//
// Strategy:
//   1. `empty_map_peek`: if `{` is immediately followed by `}`, it is an
//      empty map literal `{}`, not a block. Route to the expression parser.
//   2. `ident_map_peek`: if `{` is followed by `identifier:`, it is a map
//      literal with identifier keys (e.g. `{foo: 42}`). Route to expression.
//   3. `{` alone (no further peek): treat it as a block.
//   4. `dsl::else_`: any other expression start (non-`{`). Route to expression.
//
// Computed-key maps `{[expr]: value}` CANNOT be distinguished from a block
// containing an array `{[x]}` within bounded lookahead, so they require
// explicit parentheses when used as a lambda body: `fn -> ({[k]: v})`.
struct lambda_body
{
    static constexpr auto rule = [] {
        // Detect `{}` (empty map) and `{identifier:` (identifier-key map)
        // before the generic `{` → block branch, so they parse as expressions.
        // Note: `{[expr]:` (computed-key map) cannot be distinguished from
        // `{[expr]}` (block with array) via bounded lookahead, so it falls
        // through to the block parser.
        auto empty_map_peek = dsl::peek(
            param_ws_nl + dsl::lit_c<'{'> + param_ws_nl + dsl::lit_c<'}'>);
        auto ident_map_peek = dsl::peek(param_ws_nl
                                        + dsl::lit_c<'{'>
                                        + param_ws_nl
                                        + dsl::ascii::alpha_underscore
                                        + dsl::while_(dsl::ascii::word)
                                        + param_ws_nl
                                        + dsl::lit_c<':'>);
        auto expr_body = param_ws_nl + dsl::recurse<expression>;
        return empty_map_peek
               >> expr_body
               | ident_map_peek
               >> expr_body
               | dsl::peek(param_ws_nl + dsl::lit_c<'{'>)
               >> param_ws_nl
               >> dsl::curly_bracketed(
                   statement_ws
                   + dsl::opt(dsl::peek(expression_start_no_nl)
                              >> dsl::recurse<statement_list>)
                   + statement_ws)
               | dsl::else_
               >> (require_expr_start_nl<expected_lambda_body>()
                   >> param_ws_nl
                   >> dsl::recurse<expression>);
    }();

    // The value callback has three overloads:
    //   - lexy::nullopt: block body was empty, return empty vector
    //   - vector<Statement::Ptr>: block body with statements
    //   - Expression::Ptr: expression body, wrap in a single-element vector
    // This unifies the representation so the Lambda AST node always holds
    // a statement list regardless of which syntax was used.
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

// =============================================================================
// With-expression operations (map / filter / foreach / reduce)
// =============================================================================
//
// `map`, `filter`, and `foreach` share the same syntactic shape:
//
//   map expr with operation_fn
//   filter expr with predicate_fn
//   foreach expr with action_fn
//
// `With_Operation<node_t>` parses the `expr with operation` part (the keyword
// has already been consumed by the caller). `With_Keyword_Expr<node_t>`
// consumes the keyword itself and then delegates to `With_Operation`.
//
// The template machinery (`with_operation_keyword`, `with_operation_name`)
// avoids code duplication: one template handles all three operations by
// selecting the right keyword and AST node type at compile time. A
// static_assert with a dependent false ensures a clear error if someone
// instantiates the template with an unsupported node type.

namespace node
{
constexpr auto kw_with = LEXY_KEYWORD("with", identifier::base);
template <typename node_t>
inline constexpr bool unsupported_with_operation_node = false;

template <typename node_t>
constexpr const char* with_operation_name()
{
    if constexpr (std::is_same_v<node_t, ast::Map>)
        return "map expression";
    else if constexpr (std::is_same_v<node_t, ast::Filter>)
        return "filter expression";
    else if constexpr (std::is_same_v<node_t, ast::Foreach>)
        return "foreach expression";
    else
    {
        static_assert(
            unsupported_with_operation_node<node_t>,
            "Unsupported with-operation node type for parser keyword dispatch");
        return "";
    }
}

template <typename node_t>
constexpr auto with_operation_keyword()
{
    if constexpr (std::is_same_v<node_t, ast::Map>)
        return LEXY_KEYWORD("map", identifier::base);
    else if constexpr (std::is_same_v<node_t, ast::Filter>)
        return LEXY_KEYWORD("filter", identifier::base);
    else if constexpr (std::is_same_v<node_t, ast::Foreach>)
        return LEXY_KEYWORD("foreach", identifier::base);
    else
    {
        static_assert(
            unsupported_with_operation_node<node_t>,
            "Unsupported with-operation node type for parser keyword dispatch");
        return LEXY_KEYWORD("with", identifier::base);
    }
}

// Parses `structure with operation`. The `structure` expression is parsed
// with dsl::recurse<expression> (not expression_nl) because these expressions
// live at statement level where newlines are not whitespace; the `with`
// keyword acts as the continuation marker.
template <typename node_t>
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
            return std::make_unique<node_t>(ast::AST_Node::no_range,
                                            std::move(structure),
                                            std::move(operation));
        });
    static constexpr auto name = with_operation_name<node_t>();
};

// Parses the full `map|filter|foreach expr with fn` form, including the
// leading keyword.
template <typename node_t>
struct With_Keyword_Expr
{
    static constexpr auto rule = [] {
        auto kw = with_operation_keyword<node_t>();
        return kw + param_ws_nl + dsl::p<With_Operation<node_t>>;
    }();
    static constexpr auto value = lexy::forward<ast::Expression::Ptr>;
    static constexpr auto name = with_operation_name<node_t>();
};

using Map_Expr = With_Keyword_Expr<ast::Map>;
using Filter = With_Keyword_Expr<ast::Filter>;
using Foreach = With_Keyword_Expr<ast::Foreach>;

// `reduce` is similar to map/filter/foreach but has an optional `init:` clause:
//
//   reduce expr with fn
//   reduce expr init: initial_value with fn
//
// The `init_clause` is optional and appears between the structure and the
// `with` keyword. When absent, lexy::nullopt is produced and the value
// callback passes std::nullopt to the AST constructor.
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
                   + init_clause
                   + param_ws_nl
                   + kw_with
                   + param_ws_nl
                   + (require_expr_start_no_nl<expected_with_expression>()
                      >> dsl::recurse<expression>));
    }();
    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](ast::Expression::Ptr structure, lexy::nullopt,
           ast::Expression::Ptr operation) {
            return std::make_unique<ast::Reduce>(
                ast::AST_Node::no_range, std::move(structure),
                std::move(operation), std::optional<ast::Expression::Ptr>{});
        },
        [](ast::Expression::Ptr structure, ast::Expression::Ptr init,
           ast::Expression::Ptr operation) {
            return std::make_unique<ast::Reduce>(
                ast::AST_Node::no_range, std::move(structure),
                std::move(operation),
                std::optional<ast::Expression::Ptr>{std::move(init)});
        });
    static constexpr auto name = "reduce expression";
};

// Lambda: `fn params -> body`. param_clause and body are separate productions;
// the `fn` keyword commits entry, then both sub-productions must succeed.
struct Lambda
{
    static constexpr auto rule = [] {
        auto kw_fn = LEXY_KEYWORD("fn", identifier::base);
        return kw_fn >> (dsl::p<lambda_param_clause> + dsl::p<lambda_body>);
    }();
    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](lambda_param_pack params, std::vector<ast::Statement::Ptr> body) {
            return std::make_unique<ast::Lambda>(
                ast::AST_Node::no_range, std::move(params.params),
                std::move(body), std::move(params.vararg),
                std::move(params.self_name));
        });
    static constexpr auto name = "lambda expression";
};

// Do block: `do { statements }`. The braces are mandatory (unlike lambda which
// has multiple body forms). The body follows the same block rules as lambda's
// block body: optional statement list inside curly braces.
struct Do_Block_Expr
{
    static constexpr auto rule = [] {
        auto kw_do = LEXY_KEYWORD("do", identifier::base);
        return kw_do
               >> dsl::curly_bracketed(
                   statement_ws
                   + dsl::opt(dsl::peek(expression_start_no_nl)
                              >> dsl::recurse<statement_list>)
                   + statement_ws);
    }();
    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](lexy::nullopt) -> ast::Expression::Ptr {
            return std::make_unique<ast::Do_Block>(
                ast::AST_Node::no_range, std::vector<ast::Statement::Ptr>{});
        },
        [](std::vector<ast::Statement::Ptr> stmts) -> ast::Expression::Ptr {
            return std::make_unique<ast::Do_Block>(ast::AST_Node::no_range,
                                                   std::move(stmts));
        });
    static constexpr auto name = "do block";
};

// =============================================================================
// If expression
// =============================================================================
//
// Syntax:
//   if cond: consequent
//   if cond: consequent elif cond2: consequent2
//   if cond: consequent else: alternate
//   if cond: consequent elif cond2: consequent2 else: alternate
//
// The `elif` / `else` tail is recursive: each `elif` or `else` is parsed by
// the `Tail` inner struct, which itself optionally recurses into another Tail.
// This models the chain as a right-recursive structure.
//
// The `else if` detection (use_elif_not_else_if error) is implemented by
// peeking inside `else_branch` for the `if` keyword immediately following
// `else`. Since Lexy is non-backtracking, we must peek BEFORE deciding to
// emit the error — the peek confirms "else" was followed by "if", which is
// the erroneous pattern.
//
// `tail` uses dsl::opt with a peek at `elif|else`: if neither follows, the
// if expression has no alternative and produces lexy::nullopt (mapped to
// an If node with no alternate). The peek is necessary because the `elif`/
// `else` keywords could be on the same line or the next line — param_ws_nl
// skips both horizontal whitespace and newlines in the peek.

struct If
{
    // `Tail` parses the elif/else continuation recursively. It is a separate
    // struct (not an inner lambda) so it can be referenced via dsl::recurse<>.
    struct Tail
    {
        static constexpr auto rule = [] {
            auto kw_if = LEXY_KEYWORD("if", identifier::base);
            auto kw_elif = LEXY_KEYWORD("elif", identifier::base);
            auto kw_else = LEXY_KEYWORD("else", identifier::base);

            // The tail of the tail: another elif/else may optionally follow.
            auto tail = dsl::opt(dsl::peek(param_ws_nl + (kw_elif | kw_else))
                                 >> param_ws_nl
                                 >> dsl::recurse<Tail>);

            auto elif_branch =
                kw_elif
                >> (dsl::position
                    + dsl::recurse<expression>
                    + dsl::lit_c<':'>
                    + param_ws_nl
                    + (require_expr_start_no_nl<expected_if_consequent>()
                       >> dsl::recurse<expression>)+tail);

            // Inside `else_branch`: peek for `if` to detect `else if` and emit
            // a friendly error pointing the user to `elif`. The else_/else path
            // parses `: alternate`.
            auto else_branch =
                kw_else
                >> (dsl::peek(param_ws_nl + kw_if)
                    >> dsl::error<use_elif_not_else_if>
                    | dsl::else_
                    >> (dsl::lit_c<':'>
                        + param_ws_nl
                        + (require_expr_start_no_nl<expected_if_consequent>()
                           >> dsl::recurse<expression>)));

            return elif_branch | else_branch;
        }();

        // The value callback has three overloads:
        //   - elif: (condition, consequent, lexy::nullopt) → If node with no
        //   alternate
        //   - elif: (condition, consequent, alternate) → If node with alternate
        //   - else: (alternate) → the alternate expression directly
        // The third overload "unwraps" the else branch: its single expression
        // becomes the alternate of the enclosing If node.
        static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
            [](auto after_elif, ast::Expression::Ptr condition,
               ast::Expression::Ptr consequent, lexy::nullopt) {
                auto end = consequent->source_range().end;
                // Position is after "elif" + auto-whitespace; scan back
                // past whitespace, then past the 4-char keyword.
                auto p = reinterpret_cast<const char8_t*>(&*after_elif);
                auto kw_end = skip_trailing_ws(detail::g_input_begin, p);
                auto begin = to_source_location(kw_end - 4);
                return std::make_unique<ast::If>(
                    ast::AST_Node::Source_Range{begin, end},
                    std::move(condition), std::move(consequent));
            },
            [](auto after_elif, ast::Expression::Ptr condition,
               ast::Expression::Ptr consequent,
               ast::Expression::Ptr alternate) {
                auto end = alternate->source_range().end;
                auto p = reinterpret_cast<const char8_t*>(&*after_elif);
                auto kw_end = skip_trailing_ws(detail::g_input_begin, p);
                auto begin = to_source_location(kw_end - 4);
                return std::make_unique<ast::If>(
                    ast::AST_Node::Source_Range{begin, end},
                    std::move(condition), std::move(consequent),
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
            return std::make_unique<ast::If>(ast::AST_Node::no_range,
                                             std::move(condition),
                                             std::move(consequent));
        },
        [](ast::Expression::Ptr condition, ast::Expression::Ptr consequent,
           ast::Expression::Ptr alternate) {
            return std::make_unique<ast::If>(
                ast::AST_Node::no_range, std::move(condition),
                std::move(consequent), std::move(alternate));
        });
    static constexpr auto name = "if expression";
};
} // namespace node

// Helper: create a Literal AST node wrapping a string key. Used when an
// identifier-form map key (`foo:`) is desugared to `["foo"]:` at parse time.
// Both forms produce identical AST nodes — the short form is pure syntax sugar.
inline ast::Expression::Ptr make_string_key_expr(
    std::string key,
    ast::AST_Node::Source_Range range = ast::AST_Node::no_range)
{
    auto key_value = Value::create(std::move(key));
    return std::make_unique<ast::Literal>(range, std::move(key_value));
}

// =============================================================================
// Array and map literals
// =============================================================================

// Array elements: `[expr, expr, ...]` (trailing comma allowed).
// `delimited_list` handles the `[`, optional whitespace, list, optional
// trailing comma, `]` structure.
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

// A map entry key can be either an identifier (`foo:`) or a bracketed
// expression (`[expr]:`). `map_entry_start` is the character-level peek used
// to detect the start of any map entry.
constexpr auto map_entry_start = dsl::lit_c<'['> | dsl::ascii::alpha_underscore;

// `map_key`: parses either `[expr]` (bracket_key) or `identifier` (ident_key).
// Identifier keys are immediately converted to string-Literal expressions via
// `make_string_key_expr`, so the AST is uniform regardless of key syntax.
struct map_key
{
    static constexpr auto rule = [] {
        auto bracket_key = dsl::lit_c<'['>
                           + param_ws_nl
                           + dsl::recurse<expression_nl>
                           + param_ws_nl
                           + dsl::lit_c<']'>;
        auto ident_key =
            dsl::position + dsl::p<identifier_required> + dsl::position;
        auto key =
            dsl::peek(dsl::lit_c<'['>) >> bracket_key | dsl::else_ >> ident_key;
        return key;
    }();

    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](ast::Expression::Ptr key) {
            return key;
        },
        [](auto begin_pos, std::string key, auto end_pos) {
            return make_string_key_expr(std::move(key),
                                        make_source_range(begin_pos, end_pos));
        });
    static constexpr auto name = "map key";
};

// `map_entry_rule`: factored helper for `key : value`, shared between the
// regular map_entry and the map_destructure_entry (which has a different value
// type but the same key syntax).
template <typename value_rule_t>
constexpr auto map_entry_rule(value_rule_t value_rule)
{
    return dsl::p<map_key>
           + param_ws_nl
           + dsl::lit_c<':'>
           + param_ws_nl
           + value_rule;
}

// A regular map entry: `key: value_expr`.
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

// A map destructure entry. Three forms:
//   [expr]: binding_name  — computed key, explicit binding required
//   identifier: binding_name — named key, explicit binding
//   identifier            — shorthand: key name used as binding name
//
// Cannot reuse `map_key` here because the shorthand path needs the identifier
// string directly (before conversion to a Literal node) to use as the binding.
template <bool exported>
struct map_destructure_entry_impl
{
    static constexpr auto rule = [] {
        // Computed key [expr]: pattern
        auto bracket_key_expr = dsl::lit_c<'['>
                                + param_ws_nl
                                + dsl::recurse<expression_nl>
                                + param_ws_nl
                                + dsl::lit_c<']'>;
        auto computed = dsl::peek(dsl::lit_c<'['>)
                        >> (bracket_key_expr
                            + param_ws_nl
                            + dsl::lit_c<':'>
                            + param_ws_nl
                            + dsl::recurse<destructure_pattern_impl<exported>>);

        // Named key: peek for `:` to distinguish explicit binding from
        // shorthand.
        auto explicit_binding =
            dsl::opt(dsl::peek(param_ws_nl + dsl::lit_c<':'>)
                     >> (param_ws_nl
                         + dsl::lit_c<':'>
                         + param_ws_nl
                         + dsl::recurse<destructure_pattern_impl<exported>>));
        auto named = dsl::else_
                     >> (dsl::position
                         + dsl::p<identifier_required>
                         + dsl::position
                         + explicit_binding);

        return computed | named;
    }();

    static constexpr auto value = lexy::callback<ast::Destructure_Map::Element>(
        // Computed key with pattern binding.
        [](ast::Expression::Ptr key, ast::Destructure::Ptr dest) {
            return ast::Destructure_Map::Element{std::move(key),
                                                 std::move(dest)};
        },
        // Named key with explicit pattern binding.
        [](auto key_begin, std::string key, auto key_end,
           ast::Destructure::Ptr dest) {
            return ast::Destructure_Map::Element{
                make_string_key_expr(std::move(key),
                                     make_source_range(key_begin, key_end)),
                std::move(dest)};
        },
        // Shorthand: key name used as binding name.
        [](auto key_begin, std::string key, auto key_end, lexy::nullopt) {
            return ast::Destructure_Map::Element{
                make_string_key_expr(key,
                                     make_source_range(key_begin, key_end)),
                std::make_unique<ast::Destructure_Binding>(
                    make_source_range(key_begin, key_end),
                    key == "_" ? std::optional<std::string>{}
                               : std::optional<std::string>{std::string{key}},
                    exported)};
        });
    static constexpr auto name = "map destructure entry";
};

// `map_entries`: the full `{key: value, ...}` literal.
// `map_entry_start` gates the list so an empty `{}` produces an empty vector.
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

// `map_destructure_entries`: `{key: pattern, ...}` in a def destructure.
// Trailing comma is allowed: `{a, b,}`.
template <bool exported>
struct map_destructure_entries_impl
{
    static constexpr auto rule = [] {
        auto entry_start = dsl::peek(map_entry_start);
        auto item = entry_start >> dsl::p<map_destructure_entry_impl<exported>>;
        auto sep = comma_sep_nl_after(map_entry_start | dsl::lit_c<'}'>);
        auto list = dsl::list(item, dsl::trailing_sep(sep));
        return braced_list(map_entry_start, list);
    }();
    static constexpr auto value =
        list_or_empty<ast::Destructure_Map::Element>();
    static constexpr auto name = "map destructure";
};

// Thin node:: wrappers for array and map literals (consistent with other
// primary expression nodes that live in the node:: namespace).
namespace node
{
struct Array
{
    static constexpr auto rule = dsl::p<array_elements>;
    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](std::vector<ast::Expression::Ptr> elems) {
            return std::make_unique<ast::Array_Constructor>(
                ast::AST_Node::no_range, std::move(elems));
        });
    static constexpr auto name = "array literal";
};

struct Map
{
    static constexpr auto rule = dsl::p<map_entries>;
    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](std::vector<ast::Map_Constructor::KV_Pair> pairs) {
            return std::make_unique<ast::Map_Constructor>(
                ast::AST_Node::no_range, std::move(pairs));
        });
    static constexpr auto name = "map literal";
};
} // namespace node

// =============================================================================
// Match patterns
// =============================================================================
//
// Match patterns are the left-hand side of `match` arms. They are a separate
// AST hierarchy from destructure patterns -- destructure is "always succeed"
// while match is "fallible try_match." The shapes are similar enough that
// these productions parallel the destructure productions, but the AST nodes
// produced (Match_Binding, Match_Value, Match_Array, Match_Map) live in their
// own subtree.
//
// Surface syntax:
//   foo            -- bind to `foo`
//   _              -- discard
//   foo is Int     -- bind + type-check (`is TYPE` only on bindings)
//   _ is Int       -- discard + type-check
//   42             -- value pattern (literal)
//   "hi"           -- value pattern (literal)
//   true / null    -- value pattern (literal)
//   (expr)         -- value pattern (parenthesized expression)
//   [a, b, ...rs]  -- array pattern with optional ...rest / ..._
//   {key: pat}     -- map pattern with optional shorthand and computed keys

// Forward declarations: match patterns are mutually recursive (a Match_Array
// can contain other patterns, etc.).
struct match_pattern;
struct match_array_pattern;
struct match_map_pattern;

// Type constraint sub-productions: one per Type_Constraint enum value.
// Each matches its keyword as a complete word (via LEXY_KEYWORD's word
// boundary check) and produces the corresponding constant.
//
// Type constraint names are *contextual* keywords -- they're only treated
// specially when they appear after `is` in a match pattern. They're NOT in
// the global FROST_X_KEYWORDS, so users can still name regular bindings
// `Int`, `Float`, etc. (though that's strongly discouraged).
//
// Pushing the set into Lexy this way means a typo like `is Itn` produces
// a Lexy-level error pointing at the bad token, rather than parsing as a
// generic identifier and only failing at AST construction time.
#define X(NAME)                                                                \
    struct match_tc_##NAME                                                     \
    {                                                                          \
        static constexpr auto rule = LEXY_KEYWORD(#NAME, identifier::base);    \
        static constexpr auto value =                                          \
            lexy::callback<ast::Type_Constraint>([] {                          \
                return ast::Type_Constraint::NAME;                             \
            });                                                                \
    };
X_TYPE_CONSTRAINTS
#undef X

// `is TYPE` clause: the keyword `is`, then exactly one of the type
// constraint keywords. Lexy emits its own "expected one of: ..." error if
// none of the keywords match.
struct match_type_constraint
{
    static constexpr auto rule = [] {
        auto kw_is = LEXY_KEYWORD("is", identifier::base);
        auto choice =
#define X(NAME) dsl::p<match_tc_##NAME> |
            X_TYPE_CONSTRAINTS
#undef X
                dsl::error<expected_type_constraint>;
        return kw_is + param_ws_nl + choice;
    }();
    static constexpr auto value = lexy::forward<ast::Type_Constraint>;
    static constexpr auto name = "type constraint";
};

// Binding pattern: `_ | identifier` followed by optional `is TYPE`.
// `_` becomes a discard (Match_Binding with std::nullopt for the name).
// Anything else is a real bound name. Type constraint is optional.
struct match_binding_pattern
{
    static constexpr auto rule = [] {
        auto opt_constraint =
            dsl::opt(dsl::peek(param_ws + LEXY_KEYWORD("is", identifier::base))
                     >> (param_ws + dsl::p<match_type_constraint>));
        return dsl::position
               + dsl::p<identifier_required>
               + opt_constraint
               + dsl::position;
    }();
    static constexpr auto value = lexy::callback<ast::Match_Pattern::Ptr>(
        [](auto begin, std::string name, lexy::nullopt,
           auto end) -> ast::Match_Pattern::Ptr {
            std::optional<std::string> opt_name =
                name == "_" ? std::nullopt
                            : std::optional<std::string>{std::move(name)};
            return std::make_unique<ast::Match_Binding>(
                make_source_range(begin, end), std::move(opt_name),
                std::nullopt);
        },
        [](auto begin, std::string name, ast::Type_Constraint constraint,
           auto end) -> ast::Match_Pattern::Ptr {
            std::optional<std::string> opt_name =
                name == "_" ? std::nullopt
                            : std::optional<std::string>{std::move(name)};
            return std::make_unique<ast::Match_Binding>(
                make_source_range(begin, end), std::move(opt_name), constraint);
        });
    static constexpr auto name = "match binding pattern";
};

// Value pattern: a primitive literal OR a parenthesized expression. The
// resulting expression is wrapped in a Match_Value node and compared
// against the match target at runtime via Frost equality.
//
// `node::Literal` builds its AST node with `no_range` (it relies on the
// surrounding `primary_expression` to fix the range up). Since we consume it
// directly here, we must set the inner expression's range ourselves so the
// finished AST never contains `no_range` nodes. For the parenthesized branch,
// the inner expression already has its own range from `primary_expression`,
// so we leave it alone.
struct match_value_pattern
{
    static constexpr auto rule = [] {
        auto paren_expr = dsl::lit_c<'('>
                          + param_ws_nl
                          + dsl::recurse<expression_nl>
                          + param_ws_nl
                          + dsl::lit_c<')'>;
        auto paren_value = dsl::peek(dsl::lit_c<'('>) >> paren_expr;
        auto format_value =
            dsl::peek(dsl::lit_c<'$'>) >> dsl::p<node::Format_String>;
        auto literal_value = dsl::p<node::Literal>;
        return dsl::position
               + (paren_value | format_value | literal_value)
               + dsl::position;
    }();
    static constexpr auto value = lexy::callback<ast::Match_Pattern::Ptr>(
        [](auto begin, ast::Expression::Ptr expr, auto end) {
            auto range = make_source_range(begin, end);
            // `node::Literal`, `node::Format_String`, and `node::Name_Lookup`
            // all build their AST nodes with `no_range`, relying on the
            // surrounding `primary_expression` to fix the range up. We
            // consume them directly, so we have to assign the range here
            // when it's still `no_range`. The parenthesized branch already
            // has its own range from `primary_expression`, so we leave it
            // alone.
            if (expr->source_range().begin.line == 0)
                expr->set_source_range(range);
            return std::make_unique<ast::Match_Value>(range, std::move(expr));
        });
    static constexpr auto name = "match value pattern";
};

// Top-level pattern dispatch. Order matters:
//   1. `[` -> array pattern
//   2. `{` -> map pattern
//   3. `(` -> parenthesized value pattern
//   4. literal-starting tokens (digit, quote, $, true/false/null kw) -> value
//   5. identifier or `_` -> binding pattern
struct match_pattern
{
    static constexpr auto rule = [] {
        auto kw_true = LEXY_KEYWORD("true", identifier::base);
        auto kw_false = LEXY_KEYWORD("false", identifier::base);
        auto kw_null = LEXY_KEYWORD("null", identifier::base);

        auto literal_lookahead = dsl::peek(kw_true
                                           | kw_false
                                           | kw_null
                                           | dsl::digit<>
                                           | dsl::lit_c<'"'>
                                           | dsl::lit_c<'\''>
                                           | dsl::lit_c<'$'>);

        auto array_branch =
            dsl::peek(dsl::lit_c<'['>) >> dsl::recurse<match_array_pattern>;
        auto map_branch =
            dsl::peek(dsl::lit_c<'{'>) >> dsl::recurse<match_map_pattern>;
        auto paren_branch =
            dsl::peek(dsl::lit_c<'('>) >> dsl::p<match_value_pattern>;
        auto literal_branch = literal_lookahead >> dsl::p<match_value_pattern>;
        auto binding_branch = dsl::peek(dsl::ascii::alpha_underscore)
                              >> dsl::p<match_binding_pattern>;

        return array_branch
               | map_branch
               | paren_branch
               | literal_branch
               | binding_branch
               | dsl::error<expected_match_pattern>;
    }();
    static constexpr auto value = lexy::forward<ast::Match_Pattern::Ptr>;
    static constexpr auto name = "match pattern";
};

// Rest clause for array patterns: `...identifier` or `..._`. Lives as its
// own production so it can be referenced from `dsl::p<...>`.
struct match_rest_clause
{
    static constexpr auto rule =
        LEXY_LIT("...") + param_ws_nl + dsl::p<identifier_required>;
    static constexpr auto value =
        lexy::callback<ast::Match_Array::Rest>([](std::string name) {
            if (name == "_")
                return ast::Match_Array::Rest{std::nullopt};
            return ast::Match_Array::Rest{std::move(name)};
        });
    static constexpr auto name = "rest clause";
};

// Pattern-start lookahead used by both `match_pattern_list` and the
// surrounding array payload. Includes everything that can begin a pattern;
// crucially excludes `...` so the rest clause stops the list cleanly.
constexpr auto match_pattern_start_char = dsl::ascii::alpha_underscore
                                          | dsl::lit_c<'['>
                                          | dsl::lit_c<'{'>
                                          | dsl::lit_c<'('>
                                          | dsl::digit<>
                                          | dsl::lit_c<'"'>
                                          | dsl::lit_c<'\''>
                                          | dsl::lit_c<'$'>;

// Pattern list for array patterns. Separate production so its value is a
// `lexy::as_list` sink, which the array payload consumes as a vector.
//
// Two interacting constraints on the separator:
//   1. The list must end gracefully before `, ...rest` so the payload can
//      consume the comma as part of the rest clause. We achieve this by
//      having the separator's peek refuse to commit when the next character
//      is `.`.
//   2. A trailing comma is allowed: `[a, b,]`. We use `dsl::trailing_sep`
//      and broaden the peek to also accept the closing `]` after the comma.
struct match_pattern_list
{
    static constexpr auto rule = [] {
        auto after = match_pattern_start_char | dsl::lit_c<']'>;
        auto sep = comma_sep_nl_after(after);
        // `dsl::trailing_sep` requires the item to be a branch rule, so we
        // wrap `match_pattern` in a peek-guarded branch.
        auto item =
            dsl::peek(match_pattern_start_char) >> dsl::recurse<match_pattern>;
        return dsl::list(item, dsl::trailing_sep(sep));
    }();
    static constexpr auto value =
        lexy::as_list<std::vector<ast::Match_Pattern::Ptr>>;
    static constexpr auto name = "match patterns";
};

// Pack: the parsed payload of an array pattern -- the element patterns and
// (optionally) the rest clause.
struct match_array_pack
{
    std::vector<ast::Match_Pattern::Ptr> elements;
    std::optional<ast::Match_Array::Rest> rest;
};

// Array pattern payload: the contents of `[...]`. Mirrors
// `destructure_payload_impl`'s structure.
//
// A trailing comma after the last element is permitted only where another
// element could syntactically follow -- i.e., after a normal pattern in
// `[a, b,]`. A rest clause is the syntactic terminator of the array, so
// `[a, b, ...rest,]` and `[...rest,]` are NOT accepted. The pattern-list
// case is handled by `match_pattern_list`'s `trailing_sep`.
struct match_array_payload
{
    static constexpr auto rule = [] {
        auto rest_branch =
            dsl::peek(LEXY_LIT("...")) >> dsl::p<match_rest_clause>;
        // patterns then optional `, ...rest`. The pattern list's separator
        // bails out when it sees a `, ...`, leaving the comma for us to
        // consume here.
        auto patterns_then_rest =
            dsl::p<match_pattern_list>
            + dsl::opt(
                dsl::peek(param_ws_nl
                          + dsl::lit_c<','>
                          + param_ws_nl
                          + LEXY_LIT("..."))
                >> (param_ws_nl + dsl::lit_c<','> + param_ws_nl + rest_branch));
        auto rest_only = rest_branch;
        auto patterns_only =
            dsl::peek(match_pattern_start_char) >> patterns_then_rest;
        return dsl::opt(rest_only | patterns_only);
    }();
    static constexpr auto value = lexy::callback<match_array_pack>(
        [](lexy::nullopt) {
            return match_array_pack{};
        },
        [](ast::Match_Array::Rest rest) {
            return match_array_pack{{}, std::move(rest)};
        },
        [](std::vector<ast::Match_Pattern::Ptr> elems, lexy::nullopt) {
            return match_array_pack{std::move(elems), std::nullopt};
        },
        [](std::vector<ast::Match_Pattern::Ptr> elems,
           ast::Match_Array::Rest rest) {
            return match_array_pack{std::move(elems), std::move(rest)};
        });
    static constexpr auto name = "match array payload";
};

// Array pattern: `[pat, pat, ..., ...rest?]`.
struct match_array_pattern
{
    static constexpr auto rule = dsl::position
                                 + dsl::lit_c<'['>
                                 + param_ws_nl
                                 + dsl::p<match_array_payload>
                                 + param_ws_nl
                                 + dsl::lit_c<']'>
                                 + dsl::position;
    static constexpr auto value = lexy::callback<ast::Match_Pattern::Ptr>(
        [](auto begin, match_array_pack pack, auto end) {
            return std::make_unique<ast::Match_Array>(
                make_source_range(begin, end), std::move(pack.elements),
                std::move(pack.rest));
        });
    static constexpr auto name = "match array pattern";
};

// Map pattern entry. Three forms:
//   [expr]: pattern        -- computed key with explicit pattern
//   identifier: pattern    -- named key with explicit pattern
//   identifier             -- shorthand: key name doubles as binding name
//   identifier is TYPE     -- shorthand with type constraint
//
// All forms desugar to a Match_Map::Element with an Expression key and a
// Match_Pattern sub-pattern. Identifier keys become string Literal
// expressions. The shorthand `is TYPE` and an explicit `: pattern` are
// mutually exclusive: `{foo is Int}` is shorthand-with-constraint, and
// `{foo: bar is Int}` is explicit-with-the-pattern's-own-constraint.
struct match_map_entry
{
    static constexpr auto rule = [] {
        auto bracket_key_expr = dsl::lit_c<'['>
                                + param_ws_nl
                                + dsl::recurse<expression_nl>
                                + param_ws_nl
                                + dsl::lit_c<']'>;
        auto computed = dsl::peek(dsl::lit_c<'['>)
                        >> (bracket_key_expr
                            + param_ws_nl
                            + dsl::lit_c<':'>
                            + param_ws_nl
                            + dsl::recurse<match_pattern>);

        // Named key: an identifier followed by exactly one of:
        //   - `: pattern` (explicit form)
        //   - `is TYPE`   (shorthand with constraint)
        //   - nothing     (plain shorthand)
        auto explicit_form = dsl::peek(param_ws_nl + dsl::lit_c<':'>)
                             >> (param_ws_nl
                                 + dsl::lit_c<':'>
                                 + param_ws_nl
                                 + dsl::recurse<match_pattern>);
        auto shorthand_constraint_form =
            dsl::peek(param_ws + LEXY_KEYWORD("is", identifier::base))
            >> (param_ws + dsl::p<match_type_constraint>);
        auto trailer = dsl::opt(explicit_form | shorthand_constraint_form);

        auto named = dsl::else_
                     >> (dsl::position
                         + dsl::p<identifier_required>
                         + dsl::position
                         + trailer);

        return computed | named;
    }();

    static constexpr auto value = lexy::callback<ast::Match_Map::Element>(
        // Computed key: [expr]: pattern
        [](ast::Expression::Ptr key, ast::Match_Pattern::Ptr pat) {
            return ast::Match_Map::Element{std::move(key), std::move(pat)};
        },
        // Named key, explicit pattern: identifier: pattern
        [](auto key_begin, std::string key, auto key_end,
           ast::Match_Pattern::Ptr pat) {
            return ast::Match_Map::Element{
                make_string_key_expr(std::move(key),
                                     make_source_range(key_begin, key_end)),
                std::move(pat)};
        },
        // Plain shorthand: `{foo}` -> `{['foo']: Match_Binding("foo")}`.
        [](auto key_begin, std::string key, auto key_end, lexy::nullopt) {
            auto key_range = make_source_range(key_begin, key_end);
            std::optional<std::string> bind_name =
                key == "_" ? std::nullopt : std::optional<std::string>{key};
            return ast::Match_Map::Element{
                make_string_key_expr(std::move(key), key_range),
                std::make_unique<ast::Match_Binding>(
                    key_range, std::move(bind_name), std::nullopt)};
        },
        // Shorthand with constraint: `{foo is Int}` ->
        // `{['foo']: Match_Binding("foo", Int)}`.
        [](auto key_begin, std::string key, auto key_end,
           ast::Type_Constraint constraint) {
            auto key_range = make_source_range(key_begin, key_end);
            std::optional<std::string> bind_name =
                key == "_" ? std::nullopt : std::optional<std::string>{key};
            return ast::Match_Map::Element{
                make_string_key_expr(std::move(key), key_range),
                std::make_unique<ast::Match_Binding>(
                    key_range, std::move(bind_name), constraint)};
        });
    static constexpr auto name = "match map entry";
};

struct match_map_pattern
{
    struct entries
    {
        static constexpr auto rule = [] {
            auto entry_start = dsl::peek(map_entry_start);
            auto item = entry_start >> dsl::p<match_map_entry>;
            // Trailing comma is allowed: `{a, b,}`. Broaden the separator's
            // peek to accept the closing `}` after the comma, then use
            // `dsl::trailing_sep`.
            auto sep = comma_sep_nl_after(map_entry_start | dsl::lit_c<'}'>);
            auto list = dsl::list(item, dsl::trailing_sep(sep));
            return braced_list(map_entry_start, list);
        }();
        static constexpr auto value = list_or_empty<ast::Match_Map::Element>();
        static constexpr auto name = "match map entries";
    };

    static constexpr auto rule =
        dsl::position + dsl::p<entries> + dsl::position;
    static constexpr auto value = lexy::callback<ast::Match_Pattern::Ptr>(
        [](auto begin, std::vector<ast::Match_Map::Element> elems, auto end) {
            return std::make_unique<ast::Match_Map>(
                make_source_range(begin, end), std::move(elems));
        });
    static constexpr auto name = "match map pattern";
};

// A single match arm: `pattern (if: guard)? => result`.
//
// Newlines are permitted before/after `=>` so users can break long results
// onto a new line.
struct match_arm
{
    static constexpr auto rule = [] {
        auto kw_if = LEXY_KEYWORD("if", identifier::base);
        auto guard_clause = dsl::opt(
            dsl::peek(param_ws_nl + kw_if)
            >> (param_ws_nl
                + kw_if
                + param_ws_nl
                + dsl::lit_c<':'>
                + param_ws_nl
                + (require_expr_start_nl<expected_match_guard_expression>()
                   >> dsl::recurse<expression_nl>)));
        auto arrow = LEXY_LIT("=>");
        return dsl::p<match_pattern>
               + guard_clause
               + param_ws_nl
               + dsl::must(arrow).error<expected_match_arrow>
               + param_ws_nl
               + (require_expr_start_nl<expected_match_arm_result>()
                  >> dsl::recurse<expression_nl>);
    }();
    static constexpr auto value = lexy::callback<ast::Match::Arm>(
        [](ast::Match_Pattern::Ptr pat, lexy::nullopt,
           ast::Expression::Ptr result) {
            return ast::Match::Arm{.pattern = std::move(pat),
                                   .guard = std::nullopt,
                                   .result = std::move(result)};
        },
        [](ast::Match_Pattern::Ptr pat, ast::Expression::Ptr guard,
           ast::Expression::Ptr result) {
            return ast::Match::Arm{.pattern = std::move(pat),
                                   .guard = std::move(guard),
                                   .result = std::move(result)};
        });
    static constexpr auto name = "match arm";
};

// `match_arm_list`: comma-separated arms with an optional trailing comma.
// Lives as a separate production so its value is `lexy::as_list<vector>`,
// which the outer Match production consumes as a single vector argument.
struct match_arm_list
{
    static constexpr auto rule = [] {
        auto item = dsl::peek(match_pattern_start_char) >> dsl::p<match_arm>;
        return dsl::list(item, dsl::trailing_sep(comma_sep_nl));
    }();
    static constexpr auto value = lexy::as_list<std::vector<ast::Match::Arm>>;
    static constexpr auto name = "match arms";
};

namespace node
{
// `match TARGET { ARM, ARM, ... }` -- the top-level match expression.
//
// Arms are required to be comma-separated even when one-arm-per-line. A
// trailing comma is permitted. An empty arm list is accepted at the parser
// level (the constructed Match node would simply return null at evaluation
// time).
struct Match
{
    static constexpr auto rule = [] {
        auto kw_match = LEXY_KEYWORD("match", identifier::base);
        auto arms_clause = param_ws_nl
                           + dsl::opt(dsl::peek(match_pattern_start_char)
                                      >> dsl::p<match_arm_list>)
                           + param_ws_nl
                           + dsl::lit_c<'}'>;
        return kw_match
               >> (param_ws_nl
                   + (require_expr_start_no_nl<expected_match_target>()
                      >> dsl::recurse<expression>)+param_ws_nl
                   + dsl::lit_c<'{'>
                   + arms_clause);
    }();
    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](ast::Expression::Ptr target, lexy::nullopt) {
            return std::make_unique<ast::Match>(ast::AST_Node::no_range,
                                                std::move(target),
                                                std::vector<ast::Match::Arm>{});
        },
        [](ast::Expression::Ptr target, std::vector<ast::Match::Arm> arms) {
            return std::make_unique<ast::Match>(
                ast::AST_Node::no_range, std::move(target), std::move(arms));
        });
    static constexpr auto name = "match expression";
};
} // namespace node

// =============================================================================
// Primary expression (the dispatch table)
// =============================================================================
//
// `primary_expression` is the "atom" at the base of the expression precedence
// tower. It dispatches to the appropriate sub-expression based on the next
// token, using a series of `dsl::peek(...) >> sub_rule` branches.
//
// The ordering is significant in a few cases:
//
//   - `elif` and `else` are checked BEFORE `Name_Lookup` and trigger custom
//     errors. Without these branches they would fall through to `Name_Lookup`,
//     which rejects reserved words, then to `expected_expression` — giving no
//     indication of what the user meant.
//
//   - `Name_Lookup` comes near the end: it is the fallback for any bare word
//     not claimed by a keyword peek.
//
//   - The final `dsl::error<expected_expression>` catches anything not matched
//     by the earlier branches.
//
// Note: there is no `dsl::else_` before the error — the Literal production
// handles many token types internally, and Name_Lookup handles the identifier
// fallback. The final dsl::error is an unconditional failure for unrecognized
// input (e.g. an operator where an expression was expected).

struct primary_expression
{
    static constexpr auto rule =
        dsl::position
        + ((dsl::peek(dsl::lit_c<'('>)
            >> (dsl::lit_c<'('>
                + param_ws_nl
                + dsl::recurse<expression_nl>
                + param_ws_nl
                + dsl::lit_c<')'>))
           | dsl::peek(LEXY_KEYWORD("if", identifier::base))
           >> dsl::p<node::If>
           | dsl::peek(LEXY_KEYWORD("fn", identifier::base))
           >> dsl::p<node::Lambda>
           | dsl::peek(LEXY_KEYWORD("do", identifier::base))
           >> dsl::p<node::Do_Block_Expr>
           | dsl::peek(LEXY_KEYWORD("map", identifier::base))
           >> dsl::p<node::Map_Expr>
           | dsl::peek(LEXY_KEYWORD("filter", identifier::base))
           >> dsl::p<node::Filter>
           | dsl::peek(LEXY_KEYWORD("reduce", identifier::base))
           >> dsl::p<node::Reduce>
           | dsl::peek(LEXY_KEYWORD("foreach", identifier::base))
           >> dsl::p<node::Foreach>
           | dsl::peek(LEXY_KEYWORD("match", identifier::base))
           >> dsl::p<node::Match>
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
           | dsl::error<expected_expression>)+dsl::position;
    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        [](auto begin, ast::Expression::Ptr expr, auto end) {
            if (expr)
                expr->set_source_range(make_source_range(begin, end));
            return expr;
        });
    static constexpr auto name = "expression";
};

// Call argument list: the contents of `(args...)` after a function call `f(`.
// The opening `(` has already been consumed by the call operator. The rule
// handles both empty `()` (zero args) and non-empty arg lists with comma
// separators, allowing newlines inside the argument list.
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

// =============================================================================
// Expression production (operator precedence tower)
// =============================================================================
//
// `expression_impl<allow_nl>` inherits from lexy::expression_production, which
// implements a Pratt-style operator-precedence parser. The grammar is expressed
// as a tower of nested structs, each representing one precedence level:
//
//   postfix  — highest: index (`[`), call (`(`), dot (`.`), threaded call (`@`)
//   prefix   — unary: negation (`-`), logical not (`not`)
//   product  — *, /, %
//   sum      — +, -
//   compare  — <, <=, >, >= (non-associative: `a < b < c` is an error)
//   equal    — ==, != (non-associative)
//   logical_and — `and`
//   logical_or  — `or`  ← this is `using operation = ...`
//
// Each level's struct specifies:
//   `op`:      the operator token(s), each tagged with a type (op_add, etc.)
//   `operand`: the next-lower precedence level (or `dsl::atom` for postfix)
//
// The `value` callback of expression_impl receives the operator tag struct as
// a type argument and dispatches to the correct AST node via `requires`
// constraints:
//   - Tags with `static constexpr ast::Unary_Op op` → ast::Unop
//   - Tags with `static constexpr ast::Binary_Op op` → ast::Binop
//   - Special tags (op_index, op_call, op_dot, op_threaded_call) → their own
//     AST nodes with custom constructor logic.
//
// THREADED CALL (`@`):
//   Parsed as a postfix operator that consumes `callee(args)` on the right,
//   then inserts the left-hand side as the first argument in the value
//   callback.
//
//   The `threaded_call::callee` sub-production is a mini expression parser
//   that supports only index and dot postfix (no calls). This restricts the
//   right-hand side of `@` to a callable path expression, preventing
//   `x @ f(a)(b)` from parsing as `(x @ f(a))(b)`.
//
// COMPARISON OPERATORS:
//   `compare` uses dsl::infix_op_single instead of dsl::infix_op_left. This
//   makes comparisons non-associative: `a < b < c` is a parse error rather
//   than silently parsing as `(a < b) < c` (which would be meaningless for
//   most types). The user must parenthesize if chaining is intentional.
//
// WHITESPACE:
//   The `allow_nl` template parameter controls whether newlines are skipped.
//   Inside delimiters (arrays, call args, etc.) newlines are harmless;
//   at statement level they terminate the statement.

template <bool allow_nl>
struct expression_impl : lexy::expression_production
{
    static constexpr auto whitespace = [] {
        if constexpr (allow_nl)
            return ws_expr_nl;
        else
            return ws_no_nl;
    }();
    static constexpr auto atom = dsl::p<primary_expression>;
    static constexpr auto name = "expression";

    // Operator tag types. Each is an empty struct; their `op` member (where
    // present) is the ast::Unary_Op or ast::Binary_Op enum value that will be
    // stored in the AST node. The expression_impl value callback uses the tag
    // type itself (via template argument deduction) to select the right AST
    // constructor.
    struct op_neg
    {
        static constexpr ast::Unary_Op op = ast::Unary_Op::NEGATE;
    };
    struct op_not
    {
        static constexpr ast::Unary_Op op = ast::Unary_Op::NOT;
    };
    // These three postfix operators produce compound results (index expression,
    // call, dot access), not a simple Binary_Op, so they have no `op` member.
    struct op_index
    {
    };
    struct op_call
    {
    };
    struct op_dot
    {
    };
    struct op_threaded_call
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

    // `threaded_call`: the right-hand side of `@`.
    // Parsed as: callee_expr `(` args `)`.
    // `callee` is a mini expression that supports only index and dot postfix —
    // this restricts what can appear as the right-hand callee of `@` and avoids
    // ambiguities with nested calls.
    struct threaded_call
    {
        struct callee : lexy::expression_production
        {
            static constexpr auto whitespace = [] {
                if constexpr (allow_nl)
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
                   ast::Expression::Ptr index_expr, auto end_pos) {
                    auto range = ast::AST_Node::Source_Range{
                        lhs->source_range().begin, inclusive_end_loc(end_pos)};
                    return std::make_unique<ast::Index>(range, std::move(lhs),
                                                        std::move(index_expr));
                },
                [](ast::Expression::Ptr lhs, op_dot, auto key_begin,
                   std::string key, auto end_pos) {
                    auto range = ast::AST_Node::Source_Range{
                        lhs->source_range().begin, inclusive_end_loc(end_pos)};
                    auto key_range = make_source_range(key_begin, end_pos);
                    return std::make_unique<ast::Index>(
                        range, std::move(lhs),
                        make_string_key_expr(std::move(key), key_range));
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
        static constexpr auto name = "threaded call";
    };

    // POSTFIX operators (highest precedence):
    //   op_index:         `expr[index]`
    //   op_call:          `expr(args...)`
    //   op_dot:           `expr.field`
    //   op_threaded_call: `expr@callee(args...)`
    //
    // For op_call, the opening `(` commits entry; a dsl::must then checks
    // for either `)` (empty args) or an expression start. This ensures
    // `f(garbage` produces a "closing ')'" error rather than confusing output.
    struct postfix : dsl::postfix_op
    {
        static constexpr auto op =
            index_postfix_op<op_index>()
            / dsl::op<op_call>(
                dsl::lit_c<'('> >> (dsl::must(
                                        dsl::peek(param_ws_nl + dsl::lit_c<')'>)
                                        | expression_start_nl)
                                        .error<expected_call_arguments> >> dsl::
                                        p<call_arguments>
                                    + dsl::position))
            / dot_postfix_op<op_dot>()
            / dsl::op<op_threaded_call>(
                dsl::lit_c<'@'> >> (dsl::p<threaded_call> + dsl::position));
        using operand = dsl::atom;
    };

    // PREFIX operators: unary `-` and `not`.
    // `>>` creates a branch rule (literal as branch condition, position
    // capture as the then-clause), which Lexy accepts for operator matching.
    // The captured position is AFTER the operator token and any whitespace;
    // `prefix_op_begin` scans backward to find the true operator start.
    struct prefix : dsl::prefix_op
    {
        static constexpr auto op =
            dsl::op<op_neg>(dsl::lit_c<'-'> >> dsl::position)
            / dsl::op<op_not>(LEXY_KEYWORD("not", identifier::base)
                              >> dsl::position);
        using operand = postfix;
    };

    // MULTIPLICATIVE: *, /, %. Left-associative.
    struct product : dsl::infix_op_left
    {
        static constexpr auto op = dsl::op<op_mod>(dsl::lit_c<'%'>)
                                   / dsl::op<op_mul>(dsl::lit_c<'*'>)
                                   / dsl::op<op_div>(dsl::lit_c<'/'>);
        using operand = prefix;
    };

    // ADDITIVE: +, -. Left-associative.
    struct sum : dsl::infix_op_left
    {
        static constexpr auto op =
            dsl::op<op_add>(dsl::lit_c<'+'>) / dsl::op<op_sub>(dsl::lit_c<'-'>);
        using operand = product;
    };

    // RELATIONAL: <, <=, >, >=. Non-associative (infix_op_single).
    // LEXY_LIT not LEXY_KEYWORD: these are operator tokens, not keywords.
    // `<=` and `>=` must be listed before `<` and `>` so that Lexy does not
    // match the shorter prefix first (operator alternatives are tried in
    // order).
    struct compare : dsl::infix_op_single
    {
        static constexpr auto op = dsl::op<op_le>(LEXY_LIT("<="))
                                   / dsl::op<op_lt>(LEXY_LIT("<"))
                                   / dsl::op<op_ge>(LEXY_LIT(">="))
                                   / dsl::op<op_gt>(LEXY_LIT(">"));
        using operand = sum;
    };

    // EQUALITY: ==, !=. Non-associative.
    struct equal : dsl::infix_op_single
    {
        static constexpr auto op =
            dsl::op<op_eq>(LEXY_LIT("==")) / dsl::op<op_ne>(LEXY_LIT("!="));
        using operand = compare;
    };

    // LOGICAL AND: `and`. Left-associative. LEXY_KEYWORD (not LEXY_LIT)
    // prevents `android` from matching `and` as an operator.
    struct logical_and : dsl::infix_op_left
    {
        static constexpr auto op =
            dsl::op<op_and>(LEXY_KEYWORD("and", identifier::base));
        using operand = equal;
    };

    // LOGICAL OR: `or`. Left-associative. Top of the tower.
    struct logical_or : dsl::infix_op_left
    {
        static constexpr auto op =
            dsl::op<op_or>(LEXY_KEYWORD("or", identifier::base));
        using operand = logical_and;
    };

    using operation = logical_or;

    // The value callback is a multi-overload lexy::callback that handles every
    // combination of operator tag and operand types that the expression tower
    // can produce. C++20 requires-constraints select the right overload:
    //
    //   - (value) alone: atom with no operator — pass-through
    //   - (op_t, rhs) with op_t::op ~ Unary_Op: unary operation
    //   - (lhs, op_index, index_expr): index expression
    //   - (lhs, op_call, args): function call
    //   - (lhs, op_dot, key_string): dot access (desugared to index)
    //   - (lhs, op_threaded_call, threaded_call::result): threaded call
    //     (inserts lhs as first argument)
    //   - (lhs, op_t, rhs) with op_t::op ~ Binary_Op: binary operation
    static constexpr auto value = lexy::callback<ast::Expression::Ptr>(
        // Atom pass-through (no operator).
        [](ast::Expression::Ptr value) {
            return value;
        },
        // Prefix: unary operator.  `after_op` is the position captured after
        // the operator token and whitespace; prefix_op_begin scans backward
        // to find the true start of the operator.
        []<typename op_t>(op_t, auto after_op, ast::Expression::Ptr rhs)
            requires requires {
                { op_t::op } -> std::convertible_to<ast::Unary_Op>;
            }
                     {
                         constexpr std::size_t op_len =
                             (op_t::op == ast::Unary_Op::NEGATE) ? 1 : 3;
                         auto begin = prefix_op_begin(after_op, op_len);
                         auto range = ast::AST_Node::Source_Range{
                             begin, rhs->source_range().end};
                         return std::make_unique<ast::Unop>(
                             range, std::move(rhs), op_t::op);
                     },
                     // Postfix: index `lhs[expr]`.
                     [](ast::Expression::Ptr lhs, op_index,
                        ast::Expression::Ptr index_expr, auto end_pos) {
                         auto range = ast::AST_Node::Source_Range{
                             lhs->source_range().begin,
                             inclusive_end_loc(end_pos)};
                         return std::make_unique<ast::Index>(
                             range, std::move(lhs), std::move(index_expr));
                     },
                     // Postfix: function call `lhs(args...)`.
                     [](ast::Expression::Ptr lhs, op_call,
                        std::vector<ast::Expression::Ptr> args, auto end_pos) {
                         auto range = ast::AST_Node::Source_Range{
                             lhs->source_range().begin,
                             inclusive_end_loc(end_pos)};
                         return std::make_unique<ast::Function_Call>(
                             range, std::move(lhs), std::move(args));
                     },
                     // Postfix: dot access `lhs.field`.
                     [](ast::Expression::Ptr lhs, op_dot, auto key_begin,
                        std::string key, auto end_pos) {
                         auto range = ast::AST_Node::Source_Range{
                             lhs->source_range().begin,
                             inclusive_end_loc(end_pos)};
                         auto key_range = make_source_range(key_begin, end_pos);
                         return std::make_unique<ast::Index>(
                             range, std::move(lhs),
                             make_string_key_expr(std::move(key), key_range));
                     },
                     // Postfix: threaded call `lhs @ callee(args...)`.
                     [](ast::Expression::Ptr lhs, op_threaded_call,
                        threaded_call::result rhs, auto end_pos) {
                         auto range = ast::AST_Node::Source_Range{
                             lhs->source_range().begin,
                             inclusive_end_loc(end_pos)};
                         auto args = std::move(rhs.args);
                         args.insert(args.begin(), std::move(lhs));
                         return std::make_unique<ast::Function_Call>(
                             range, std::move(rhs.callee), std::move(args));
                     },
                     // Infix: binary operator.  Range spans from lhs begin to
                     // rhs end.
                     []<typename op_t>(ast::Expression::Ptr lhs, op_t,
                                       ast::Expression::Ptr rhs)
                         requires requires {
                             {
                                 op_t::op
                             } -> std::convertible_to<ast::Binary_Op>;
                         }
        {
            auto range = ast::AST_Node::Source_Range{lhs->source_range().begin,
                                                     rhs->source_range().end};
            return std::make_unique<ast::Binop>(range, std::move(lhs), op_t::op,
                                                std::move(rhs));
        });
};

// The two instantiations of the expression template. `expression` is used at
// statement level (newlines not skipped). `expression_nl` is used inside
// delimiters and multi-line contexts (newlines skipped as whitespace).
struct expression : expression_impl<false>
{
};

struct expression_nl : expression_impl<true>
{
};

// =============================================================================
// Definition statements (def / export def)
// =============================================================================

namespace node
{
// `define_lhs`: the left-hand side of a `def` statement.
// Three forms, all producing Destructure::Ptr:
//   def name = expr               — identifier binding
//   def [a, b, ...rest] = expr   — array destructure (recursive)
//   def {key: pattern} = expr     — map destructure (recursive)
// Templated on export_flag to thread it into destructure node constructors.
template <bool export_flag>
constexpr auto define_lhs = [] {
    return dsl::p<destructure_pattern_impl<export_flag>>;
}();

// `define_payload`: the `lhs = expr` part (after the `def` keyword).
template <bool export_flag>
constexpr auto define_payload = [] {
    return define_lhs<export_flag>
           + dsl::lit_c<'='>
           + param_ws
           + dsl::p<expression>;
}();

// `define_callback`: constructs a Define from a Destructure::Ptr and
// an Expression::Ptr. Single overload — all LHS forms produce
// Destructure::Ptr.
constexpr auto define_callback()
{
    return lexy::callback<ast::Statement::Ptr>([](ast::Destructure::Ptr dest,
                                                  ast::Expression::Ptr expr) {
        return std::make_unique<ast::Define>(ast::AST_Node::no_range,
                                             std::move(dest), std::move(expr));
    });
}

// `def name = expr`. The `def` keyword commits; then `define_payload` must
// succeed. Produces a non-exported binding.
struct Define
{
    static constexpr auto rule = [] {
        auto kw_def = LEXY_KEYWORD("def", identifier::base);
        return kw_def >> define_payload<false>;
    }();
    static constexpr auto value = define_callback();
    static constexpr auto name = "def statement";
};

// `export def name = expr`. Both keywords must appear with only horizontal
// whitespace between them (`param_ws_no_comment` — no comments, because
// `export # comment\n def` is a surprising place to hide a comment).
// Produces an exported binding.
struct Export_Def
{
    static constexpr auto rule = [] {
        auto kw_export = LEXY_KEYWORD("export", identifier::base);
        auto kw_def = LEXY_KEYWORD("def", identifier::base);
        return kw_export + param_ws_no_comment + kw_def + define_payload<true>;
    }();
    static constexpr auto value = define_callback();
    static constexpr auto name = "export def statement";
};

// `defn name(params) -> body`. Syntactic sugar for
// `def name = fn name(params) -> body`. The function name is bound as
// self_name in the Lambda, so the body can call it recursively.
// Permitted wherever `def` is (top-level and inside block bodies).
struct Defn
{
    static constexpr auto rule = [] {
        auto kw_defn = LEXY_KEYWORD("defn", identifier::base);
        return kw_defn
               >> (param_ws
                   + dsl::position
                   + dsl::p<identifier_required>
                   + dsl::position
                   + param_ws
                   + dsl::p<lambda_parameters_paren>
                   + param_ws_nl
                   + LEXY_LIT("->")
                   + dsl::p<lambda_body>
                   + dsl::position);
    }();
    static constexpr auto value = lexy::callback<ast::Statement::Ptr>(
        [](auto begin_pos, std::string name, auto name_end_pos,
           lambda_param_pack params, std::vector<ast::Statement::Ptr> body,
           auto end_pos) {
            auto lambda = std::make_unique<ast::Lambda>(
                make_source_range(begin_pos, end_pos), std::move(params.params),
                std::move(body), std::move(params.vararg), name);
            auto binding = std::make_unique<ast::Destructure_Binding>(
                make_source_range(begin_pos, name_end_pos),
                std::optional<std::string>{std::string{name}}, false);
            return std::make_unique<ast::Define>(
                ast::AST_Node::no_range, std::move(binding), std::move(lambda));
        });
    static constexpr auto name = "defn statement";
};

// `export defn name(params) -> body`. Top-level only; same desugaring as
// `Defn` but produces an exported binding.
struct Export_Defn
{
    static constexpr auto rule = [] {
        auto kw_export = LEXY_KEYWORD("export", identifier::base);
        auto kw_defn = LEXY_KEYWORD("defn", identifier::base);
        return kw_export
               + param_ws_no_comment
               + kw_defn
               + param_ws
               + dsl::position
               + dsl::p<identifier_required>
               + dsl::position
               + param_ws
               + dsl::p<lambda_parameters_paren>
               + param_ws_nl
               + LEXY_LIT("->")
               + dsl::p<lambda_body>
               + dsl::position;
    }();
    static constexpr auto value = lexy::callback<ast::Statement::Ptr>(
        [](auto begin_pos, std::string name, auto name_end_pos,
           lambda_param_pack params, std::vector<ast::Statement::Ptr> body,
           auto end_pos) {
            auto lambda = std::make_unique<ast::Lambda>(
                make_source_range(begin_pos, end_pos), std::move(params.params),
                std::move(body), std::move(params.vararg), name);
            auto binding = std::make_unique<ast::Destructure_Binding>(
                make_source_range(begin_pos, name_end_pos),
                std::optional<std::string>{std::string{name}}, true);
            return std::make_unique<ast::Define>(
                ast::AST_Node::no_range, std::move(binding), std::move(lambda));
        });
    static constexpr auto name = "export defn statement";
};
} // namespace node

// =============================================================================
// Statements
// =============================================================================

// An expression used as a statement. The expression is wrapped in a
// Statement::Ptr via implicit conversion from Expression::Ptr.
struct expression_statement
{
    static constexpr auto rule = expression_start_no_nl >> dsl::p<expression>;
    static constexpr auto value =
        lexy::callback<ast::Statement::Ptr>([](ast::Expression::Ptr expr) {
            return ast::Statement::Ptr{std::move(expr)};
        });
    static constexpr auto name = "expression statement";
};

// `statement_impl<allow_export>`: a single statement. The template parameter
// bakes the `export def`/`export defn` allowance in at compile time with zero
// runtime overhead.
//
// Ordering at top level: `export defn` is peeked before `export def` (both
// start with `export`, so the more specific peek comes first), then `defn`,
// then `def`, then expression statement. Inside block bodies the export
// branches are absent.
template <bool allow_export>
struct statement_impl
{
    static constexpr auto rule = [] {
        auto kw_defn = LEXY_KEYWORD("defn", identifier::base);
        if constexpr (allow_export)
        {
            auto kw_export = LEXY_KEYWORD("export", identifier::base);
            return param_ws
                   + dsl::position
                   + ((dsl::peek(kw_export + param_ws_no_comment + kw_defn)
                       >> dsl::p<node::Export_Defn>)
                      | (dsl::peek(kw_export) >> dsl::p<node::Export_Def>)
                      | (dsl::peek(kw_defn) >> dsl::p<node::Defn>)
                      | dsl::p<node::Define>
                      | dsl::p<expression_statement>)+dsl::position;
        }
        else
        {
            return param_ws
                   + dsl::position
                   + ((dsl::peek(kw_defn) >> dsl::p<node::Defn>)
                      | dsl::p<node::Define>
                      | dsl::p<expression_statement>)+dsl::position;
        }
    }();
    static constexpr auto value = lexy::callback<ast::Statement::Ptr>(
        [](auto begin, ast::Statement::Ptr stmt, auto end) {
            stmt->set_source_range(make_source_range(begin, end));
            return stmt;
        });
    static constexpr auto name = "statement";
};

using statement = statement_impl<false>;
using top_level_statement = statement_impl<true>;

// =============================================================================
// Statement lists
// =============================================================================
//
// A statement list is a sequence of statements separated by newlines or
// semicolons. The separator rule peeks ahead to confirm a newline or semicolon
// follows (after optional horizontal whitespace) before consuming it; this
// avoids treating the end of a block `}` as a separator and accidentally
// consuming it.
//
// The `item` branch peeks at `expression_start_no_nl` before parsing a
// statement, so an empty or trailing-separator list terminates cleanly rather
// than trying to parse a statement from the closing `}` or EOF.

template <typename stmt_t>
struct statement_list_impl
{
    static constexpr auto rule = [] {
        // The separator matches any horizontal whitespace followed by `;`,
        // `\n`, or `#` (start of a comment that spans to `\n`). The peek
        // ensures we only consume statement_ws if a genuine separator follows,
        // not if we are at the end of the input or at a closing brace.
        auto sep =
            dsl::peek(dsl::while_(no_nl_chars)
                      + (dsl::lit_c<';'> | dsl::lit_c<'\n'> | dsl::lit_c<'#'>))
            >> statement_ws;
        auto item = dsl::peek(expression_start_no_nl) >> dsl::p<stmt_t>;
        return dsl::list(item, dsl::trailing_sep(sep));
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

// =============================================================================
// Top-level program rule
// =============================================================================
//
// A Frost program is an optional list of top-level statements surrounded by
// `statement_ws` (which skips blank lines, comments, semicolons).
//
// After the statement list, before dsl::eof, we insert "helpful mistake" peeks:
//   - `&&` → "use 'and'"
//   - `||` → "use 'or'"
//   - `=`  → "use 'def'"
//
// These are checked AFTER the statement list so that they fire only when the
// statement list has consumed as much as it can and these tokens appear next.
// If they appeared inside expressions, the expression parser would have already
// handled or rejected them. Catching them here provides actionable errors for
// the most common mistakes at statement boundary (e.g. writing `x = 5`
// instead of `def x = 5`).
//
// `ws_no_nl` is the automatic whitespace for this production; it prevents
// the top-level parser from silently consuming newlines between the `eof`
// check and the end of input.

struct program
{
    static constexpr auto whitespace = ws_no_nl;
    static constexpr auto rule = statement_ws
                                 + dsl::opt(dsl::p<top_level_statement_list>)
                                 + statement_ws
                                 + (dsl::peek(LEXY_LIT("&&"))
                                    >> dsl::error<use_and_not_double_ampersand>
                                    | dsl::peek(LEXY_LIT("||"))
                                    >> dsl::error<use_or_not_double_pipe>
                                    | dsl::peek(dsl::lit_c<'='>)
                                    >> dsl::error<use_def_for_assignment>
                                    | dsl::eof);
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

// =============================================================================
// Data Expression rule
// =============================================================================
//
// A restricted form which parses exactly one expression,
// for Frost Data deserialization.
//
// Further AST validation is done on the full AST.

struct data_expression
{
    static constexpr auto rule = dsl::p<expression_nl> + dsl::eof;
    static constexpr auto value = lexy::forward<ast::Expression::Ptr>;
};

} // namespace frst::grammar

#endif
