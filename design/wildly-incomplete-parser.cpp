#include <lexy/action/parse.hpp>
#include <lexy/action/parse_as_tree.hpp>
#include <lexy/callback.hpp>
#include <lexy/dsl.hpp>
#include <lexy/dsl/brackets.hpp>
#include <lexy/input/argv_input.hpp>
#include <lexy/visualize.hpp>
#include <lexy_ext/report_error.hpp>

#include <cstdio>
#include <cstdlib>
#include <generator>
#include <iostream>
#include <memory>
#include <string>
#include <string_view>
#include <utility>

// Minimal AST for the example grammar below. This is just enough structure
// to show parse shape; it doesn't evaluate expressions.
namespace ast
{
struct Expr;
using Expr_Ptr = std::unique_ptr<Expr>;

struct Expr
{
    virtual ~Expr() = default;
    void print(std::ostream& os) const
    {
        print_impl(os, Print_Context{"", true, true});
    }

  protected:
    struct Child_Info
    {
        const Expr* node = nullptr;
        std::string_view label;
    };

    // Per-node label shown in the tree output.
    virtual std::string node_label() const = 0;
    // Yield children in order. Yield nothing for leaf nodes.
    // The optional label is used for nodes like If that annotate children.
    virtual std::generator<Child_Info> children() const
    {
        co_return;
    }

    static Child_Info make_child(const Expr_Ptr& child,
                                 std::string_view label = {})
    {
        return Child_Info{child.get(), label};
    }

  private:
    struct Print_Context
    {
        std::string_view prefix;
        bool is_last;
        bool is_root;
    };

    // Recursive printer for `tree`-style output.
    // - `prefix` is the accumulated indentation and vertical bars from parents.
    // - `is_last` controls whether this node uses └── (last) or ├── (not last).
    // - `is_root` avoids printing connectors for the top-level node.
    void print_impl(std::ostream& os, const Print_Context& context) const
    {
        const auto label = node_label();
        print_node(os, context, label);

        const auto child_prefix_ = child_prefix(context);
        Child_Info previous{};
        bool has_previous = false;
        for (auto child : children())
        {
            if (has_previous)
                print_child(os, Print_Context{child_prefix_, false, false},
                            previous);
            previous = child;
            has_previous = true;
        }

        if (has_previous)
            print_child(os, Print_Context{child_prefix_, true, false},
                        previous);
    }

    // Emit one tree line. Root prints without connectors; children use ├──/└──
    // and the accumulated prefix.
    static void print_node(std::ostream& os, const Print_Context& context,
                           std::string_view label)
    {
        if (context.is_root)
            os << label << "\n";
        else
            os << context.prefix << (context.is_last ? "└── " : "├── ") << label
               << "\n";
    }

    static void print_child(std::ostream& os, const Print_Context& context,
                            const Child_Info& child)
    {
        if (child.label.empty())
        {
            child.node->print_impl(
                os, Print_Context{context.prefix, context.is_last, false});
            return;
        }

        // Labeled children get an extra label node, then the real child
        // is printed as the only descendant of that label.
        print_node(os, context, child.label);
        const auto labeled_prefix = child_prefix(context);
        child.node->print_impl(os, Print_Context{labeled_prefix, true, false});
    }

    static std::string child_prefix(const Print_Context& context)
    {
        std::string out(context.prefix);
        // For non-root nodes, extend the prefix: keep a vertical bar if more
        // siblings follow, otherwise add whitespace.
        if (!context.is_root)
            out += (context.is_last ? "    " : "│   ");
        return out;
    }
};

struct Literal final : Expr
{
    explicit Literal(std::string value) : value_(std::move(value))
    {
    }

  protected:
    std::string node_label() const override
    {
        return "Literal(" + value_ + ")";
    }

  private:
    std::string value_;
};

struct Identifier final : Expr
{
    explicit Identifier(std::string name) : name_(std::move(name))
    {
    }

  protected:
    std::string node_label() const override
    {
        return "Identifier(" + name_ + ")";
    }

  private:
    std::string name_;
};

struct Unary final : Expr
{
    Unary(std::string op, Expr_Ptr rhs)
        : op_(std::move(op)), rhs_(std::move(rhs))
    {
    }

  protected:
    std::string node_label() const override
    {
        return "Unary(" + op_ + ")";
    }

    std::generator<Child_Info> children() const override
    {
        co_yield make_child(rhs_);
    }

  private:
    std::string op_;
    Expr_Ptr rhs_;
};

struct Binary final : Expr
{
    Binary(char op, Expr_Ptr lhs, Expr_Ptr rhs)
        : op_(op), lhs_(std::move(lhs)), rhs_(std::move(rhs))
    {
    }

  protected:
    std::string node_label() const override
    {
        return std::string("Binary(") + op_ + ")";
    }

    std::generator<Child_Info> children() const override
    {
        co_yield make_child(lhs_);
        co_yield make_child(rhs_);
    }

  private:
    char op_;
    Expr_Ptr lhs_;
    Expr_Ptr rhs_;
};

struct Index final : Expr
{
    Index(Expr_Ptr base, Expr_Ptr index)
        : base_(std::move(base)), index_(std::move(index))
    {
    }

  protected:
    std::string node_label() const override
    {
        return "Index";
    }

    std::generator<Child_Info> children() const override
    {
        co_yield make_child(base_);
        co_yield make_child(index_);
    }

  private:
    Expr_Ptr base_;
    Expr_Ptr index_;
};

struct If_Expr final : Expr
{
    // else_branch is nullptr when the optional else-clause is absent.
    If_Expr(Expr_Ptr cond, Expr_Ptr then_branch, Expr_Ptr else_branch)
        : cond_(std::move(cond)), then_(std::move(then_branch)),
          else_(std::move(else_branch))
    {
    }

  protected:
    std::string node_label() const override
    {
        return "If";
    }

    std::generator<Child_Info> children() const override
    {
        co_yield make_child(cond_, "Cond");
        co_yield make_child(then_, "Then");
        if (else_)
            co_yield make_child(else_, "Else");
    }

  private:
    Expr_Ptr cond_;
    Expr_Ptr then_;
    Expr_Ptr else_;
};
} // namespace ast

namespace grammar
{
namespace dsl = lexy::dsl;

// argv_input inserts '\0' between arguments; treat them as whitespace so
// multiple argv parts form one input string. We also allow newlines by using
// ascii::space instead of ascii::blank.
// https://lexy.foonathan.net/reference/input/argv_input/
// https://lexy.foonathan.net/reference/dsl/whitespace/
constexpr auto ws = dsl::whitespace(dsl::ascii::space | dsl::argv_separator);

struct ident
{
    static constexpr auto base =
        dsl::identifier(dsl::ascii::alpha_underscore, dsl::ascii::word);
    // Reserve keywords so they are rejected as identifiers in branch contexts,
    // and reported as reserved if matched directly.
    // https://lexy.foonathan.net/reference/dsl/identifier/
    static constexpr auto rule =
        base.reserve(LEXY_KEYWORD("else", base), LEXY_KEYWORD("elif", base),
                     LEXY_KEYWORD("if", base), LEXY_KEYWORD("abs", base));
    static constexpr auto value = lexy::as_string<std::string>;
};

struct ident_expr
{
    static constexpr auto rule = dsl::p<ident>;
    static constexpr auto value = lexy::new_<ast::Identifier, ast::Expr_Ptr>;
};

struct number_expr
{
    struct literal
    {
        static constexpr auto rule = [] {
            constexpr auto digits = dsl::digits<>;
            auto number =
                dsl::token(digits + dsl::opt(dsl::lit_c<'.'> >> digits));
            return dsl::capture(number);
        }();
        static constexpr auto value = lexy::as_string<std::string>;
    };

    static constexpr auto rule = dsl::p<literal>;
    static constexpr auto value = lexy::new_<ast::Literal, ast::Expr_Ptr>;
};

// Operator tags: empty types are enough; dsl::op<Tag>(...) will pass a Tag
// value.
struct tag_add
{
};
struct tag_sub
{
};
struct tag_mul
{
};
struct tag_div
{
};
struct tag_neg
{
};
struct tag_index
{
};

struct expr;

struct if_expr
{
    struct if_tail
    {
        static constexpr auto rule = [] {
            constexpr auto kw_elif = LEXY_KEYWORD("elif", ident::rule);
            constexpr auto kw_else = LEXY_KEYWORD("else", ident::rule);

            return (kw_else >>
                    (ws + dsl::lit_c<':'> + ws + dsl::recurse<expr>)) |
                   (kw_elif >> (ws + dsl::recurse<expr> + ws + dsl::lit_c<':'> +
                                ws + dsl::recurse<expr> +
                                dsl::opt(dsl::recurse_branch<if_tail>)));
        }();

        static constexpr auto value = lexy::callback<ast::Expr_Ptr>(
            [](ast::Expr_Ptr else_expr) { return else_expr; },
            [](ast::Expr_Ptr cond, ast::Expr_Ptr then_expr,
               ast::Expr_Ptr else_expr) {
                return std::make_unique<ast::If_Expr>(std::move(cond),
                                                      std::move(then_expr),
                                                      std::move(else_expr));
            });
    };

    static constexpr auto rule = [] {
        // LEXY_KEYWORD matches full identifiers, not just prefixes.
        // https://lexy.foonathan.net/reference/dsl/identifier/
        constexpr auto kw_if = LEXY_KEYWORD("if", ident::rule);

        // Use `kw_if >> ...` to make this a branch rule so it can appear in a
        // choice.
        // https://lexy.foonathan.net/reference/dsl/branch/
        // https://lexy.foonathan.net/reference/dsl/choice/
        // `dsl::recurse<expr>` avoids needing `expr` to be fully defined here.
        // https://lexy.foonathan.net/reference/dsl/production/
        return kw_if >> (ws + dsl::recurse<expr> + ws + dsl::lit_c<':'> + ws +
                         dsl::recurse<expr> + dsl::opt(dsl::p<if_tail>));
    }();
    // dsl::opt yields lexy::nullopt when the else branch is missing; it
    // converts to a null Expr_Ptr.
    // https://lexy.foonathan.net/reference/dsl/option/
    static constexpr auto value = lexy::callback<ast::Expr_Ptr>(
        [](ast::Expr_Ptr cond, ast::Expr_Ptr then_expr,
           ast::Expr_Ptr else_expr) {
            return std::make_unique<ast::If_Expr>(
                std::move(cond), std::move(then_expr), std::move(else_expr));
        });
};

// Expression with C-like precedence. expression_production orders precedence
// by chaining operation types through their `operand` aliases.
// https://lexy.foonathan.net/reference/dsl/expression/
//   postfix index []
//   unary prefix -
//   * /
//   + -
struct expr : lexy::expression_production
{
    static constexpr auto whitespace = ws;

    // Atom is the base operand for the expression parser.
    static constexpr auto atom = dsl::p<if_expr> | dsl::p<ident_expr> |
                                 dsl::p<number_expr> |
                                 dsl::parenthesized(dsl::p<expr>);

    struct index : dsl::postfix_op
    {
        // dsl::op() requires a literal or a branch with a literal condition.
        // The square_bracketed rule is a branch that starts with '[', so it
        // qualifies.
        // https://lexy.foonathan.net/reference/dsl/operator/
        static constexpr auto op =
            dsl::op<tag_index>(dsl::square_bracketed(dsl::p<expr>));
        using operand = dsl::atom;
    };

    struct neg : dsl::prefix_op
    {
        // Multi-character operators are fine; dsl::op just needs a
        // literal/branch.
        // https://lexy.foonathan.net/reference/dsl/operator/
        static constexpr auto op = dsl::op<tag_neg>(dsl::lit_c<'-'>);
        using operand = index;
    };

    struct product : dsl::infix_op_left
    {
        static constexpr auto op = dsl::op<tag_mul>(dsl::lit_c<'*'>) /
                                   dsl::op<tag_div>(dsl::lit_c<'/'>);
        using operand = neg;
    };

    struct sum : dsl::infix_op_left
    {
        static constexpr auto op = dsl::op<tag_add>(dsl::lit_c<'+'>) /
                                   dsl::op<tag_sub>(dsl::lit_c<'-'>);
        using operand = product;
    };

    using operation = sum;

    // Lexy calls this callback directly for atoms and operators.
    // Prefix ops pass (op, operand); postfix ops pass (operand, op).
    // https://lexy.foonathan.net/reference/dsl/expression/
    static constexpr auto value = lexy::callback<ast::Expr_Ptr>(
        [](ast::Expr_Ptr value) { return value; },
        [](tag_neg, ast::Expr_Ptr rhs) {
            return std::make_unique<ast::Unary>("-", std::move(rhs));
        },
        [](ast::Expr_Ptr lhs, tag_add, ast::Expr_Ptr rhs) {
            return std::make_unique<ast::Binary>('+', std::move(lhs),
                                                 std::move(rhs));
        },
        [](ast::Expr_Ptr lhs, tag_sub, ast::Expr_Ptr rhs) {
            return std::make_unique<ast::Binary>('-', std::move(lhs),
                                                 std::move(rhs));
        },
        [](ast::Expr_Ptr lhs, tag_mul, ast::Expr_Ptr rhs) {
            return std::make_unique<ast::Binary>('*', std::move(lhs),
                                                 std::move(rhs));
        },
        [](ast::Expr_Ptr lhs, tag_div, ast::Expr_Ptr rhs) {
            return std::make_unique<ast::Binary>('/', std::move(lhs),
                                                 std::move(rhs));
        },
        [](ast::Expr_Ptr base, tag_index, ast::Expr_Ptr index) {
            return std::make_unique<ast::Index>(std::move(base),
                                                std::move(index));
        });
};

struct root
{
    // Require full input consumption (no trailing junk).
    static constexpr auto rule = dsl::p<expr> + dsl::eof;
    static constexpr auto value = lexy::forward<ast::Expr_Ptr>;
};
} // namespace grammar

int main(int argc, char** argv)
{
    if (argc < 2)
    {
        std::cerr << "Usage: " << argv[0] << " <expression>\n";
        return 1;
    }

    // argv_input concatenates argv[1..] with '\0' separators; we treat those
    // separators as whitespace so multiple argv parts form one input string.
    // https://lexy.foonathan.net/reference/input/argv_input/
    const char* parse_tree_env = std::getenv("PARSE_TREE");
    const bool use_parse_tree =
        parse_tree_env && std::string_view(parse_tree_env) == "true";

    using Input = lexy::argv_input<>;
    Input input(argc, argv);
    if (use_parse_tree)
    {
        lexy::parse_tree_for<Input> tree;
        // parse_as_tree builds a concrete parse tree (not an AST) and discards
        // values.
        // https://lexy.foonathan.net/reference/action/parse_as_tree/
        auto tree_result = lexy::parse_as_tree<grammar::root>(
            tree, input, lexy_ext::report_error);
        if (!tree_result)
            return 1;
        // visualize() prints a human-readable parse tree; intended for
        // debugging. https://lexy.foonathan.net/reference/visualize/
        lexy::visualize(stdout, tree);
        return 0;
    }

    auto result = lexy::parse<grammar::root>(input, lexy_ext::report_error);
    if (!result)
        return 1;

    // This is the minimal AST; parse trees are only shown when PARSE_TREE=true.
    result.value()->print(std::cout);
    return 0;
}
