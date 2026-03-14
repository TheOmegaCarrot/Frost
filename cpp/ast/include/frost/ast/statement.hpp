#ifndef FROST_AST_STATEMENT_HPP
#define FROST_AST_STATEMENT_HPP

#include <generator>
#include <memory>
#include <ostream>
#include <string>
#include <string_view>

#include <fmt/format.h>

#include <frost/execution-context.hpp>

namespace frst::ast
{

//! @brief Common base class of all AST nodes / statements
class Statement
{
  public:
    using Ptr = std::unique_ptr<Statement>;

    struct Source_Location
    {
        std::size_t line;
        std::size_t column;
    };

    struct Source_Range
    {
        Source_Location begin;
        Source_Location end;
    };
    constexpr static inline Source_Location no_location{0, 0};
    constexpr static inline Source_Range no_range{no_location, no_location};

    Statement(Source_Range source_range)
        : source_range_{source_range}
    {
    }

    Statement() = delete;
    Statement(const Statement&) = delete;
    Statement(Statement&&) = delete;
    Statement& operator=(const Statement&) = delete;
    Statement& operator=(Statement&&) = delete;
    virtual ~Statement() = default;

    //! @brief Execute this statement
    std::optional<Map> execute(Execution_Context ctx) const
    {
        return do_execute(ctx);
    }

    //! @brief Print AST of this node and all descendents
    void debug_dump_ast(std::ostream& out) const;

    struct Definition
    {
        std::string name;
    };
    struct Usage
    {
        std::string name;
    };
    using Symbol_Action = std::variant<Definition, Usage>;

    virtual std::generator<Symbol_Action> symbol_sequence() const
    {
        for (const Child_Info& child : children())
            co_yield std::ranges::elements_of(child.node->symbol_sequence());
    }

    std::string node_label() const;

    // True if a node is safe to deserialize from Frost Data
    virtual bool data_safe() const
    {
        return false;
    }

    std::generator<const Statement*> walk() const
    {
        co_yield this;
        for (const Child_Info& child : children())
            co_yield std::ranges::elements_of(child.node->walk());
    }

    Source_Range source_range() const
    {
        return source_range_;
    }

    void set_source_range(Source_Range range)
    {
        source_range_ = range;
    }

  protected:
    virtual std::string do_node_label() const = 0;

    virtual std::optional<Map> do_execute(Execution_Context& ctx) const = 0;

    struct Child_Info
    {
        const Statement* node = nullptr;
        std::string label;
    };

    static Child_Info make_child(const auto& child, std::string label = {})
    {
        return Child_Info{child.get(), std::move(label)};
    }

    //! @brief Iterate over children (possibly empty)
    virtual std::generator<Child_Info> children() const
    {
        co_return;
    }

  private:
    struct Print_Context
    {
        std::string_view prefix;
        bool is_last;
        bool is_root;
    };

    void debug_dump_ast_impl(std::ostream& out,
                             const Print_Context& context) const;

    static void print_node(std::ostream& out, const Print_Context& context,
                           std::string_view label);

    static void print_child(std::ostream& out, const Print_Context& context,
                            const Child_Info& child);

    static std::string child_prefix(const Print_Context& context);

    Source_Range source_range_;
};

} // namespace frst::ast

template <>
struct fmt::formatter<frst::ast::Statement::Source_Location>
    : fmt::formatter<std::string>
{
    auto format(const frst::ast::Statement::Source_Location& loc,
                fmt::format_context& ctx) const
    {
        return fmt::format_to(ctx.out(), "{}:{}", loc.line, loc.column);
    }
};

template <>
struct fmt::formatter<frst::ast::Statement::Source_Range>
    : fmt::formatter<std::string>
{
    auto format(const frst::ast::Statement::Source_Range& range,
                fmt::format_context& ctx) const
    {
        return fmt::format_to(ctx.out(), "{}-{}", range.begin, range.end);
    }
};

#endif
