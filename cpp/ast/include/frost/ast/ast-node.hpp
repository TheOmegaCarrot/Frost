#ifndef FROST_AST_AST_NODE_HPP
#define FROST_AST_AST_NODE_HPP

#include <generator>
#include <memory>
#include <optional>
#include <ostream>
#include <string>
#include <string_view>

#include <fmt/format.h>

#include <frost/backtrace.hpp>

namespace frst::ast
{

//! @brief Common base class of all AST nodes (tree infrastructure)
class AST_Node
{
  public:
    using Ptr = std::unique_ptr<AST_Node>;

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

    AST_Node(const Source_Range& source_range)
        : source_range_{source_range}
    {
    }

    AST_Node() = delete;
    AST_Node(const AST_Node&) = delete;
    AST_Node(AST_Node&&) = delete;
    AST_Node& operator=(const AST_Node&) = delete;
    AST_Node& operator=(AST_Node&&) = delete;
    virtual ~AST_Node() = default;

    //! @brief Print AST of this node and all descendents
    void debug_dump_ast(std::ostream& out) const;

    struct Definition
    {
        std::string name;
        bool exported;
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

    std::generator<const AST_Node*> walk() const
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

    const std::shared_ptr<const std::string>& filepath() const
    {
        return filepath_;
    }

    void set_filepath(std::shared_ptr<const std::string> filepath) const
    {
        filepath_ = filepath;
    }

    struct Child_Info
    {
        const AST_Node* node = nullptr;
        std::string label;
    };

    //! @brief Iterate over children (possibly empty)
    virtual std::generator<Child_Info> children() const
    {
        co_return;
    }

  protected:
    virtual std::string do_node_label() const = 0;

    static Child_Info make_child(const auto& child, std::string label = {})
    {
        return Child_Info{child.get(), std::move(label)};
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

    // This is mutable just so that the parser can stamp nodes with a filename
    // after construction using `walk()`
    mutable std::shared_ptr<const std::string> filepath_;
};

} // namespace frst::ast

template <>
struct fmt::formatter<frst::ast::AST_Node::Source_Location>
    : fmt::formatter<std::string>
{
    auto format(const frst::ast::AST_Node::Source_Location& loc,
                fmt::format_context& ctx) const
    {
        return fmt::format_to(ctx.out(), "{}:{}", loc.line, loc.column);
    }
};

template <>
struct fmt::formatter<frst::ast::AST_Node::Source_Range>
    : fmt::formatter<std::string>
{
    auto format(const frst::ast::AST_Node::Source_Range& range,
                fmt::format_context& ctx) const
    {
        return fmt::format_to(ctx.out(), "{}-{}", range.begin, range.end);
    }
};

namespace frst::ast
{

inline std::optional<Frame_Guard> make_node_frame_guard(const AST_Node& node)
{
    auto* bt = Backtrace_State::current();
    if (not bt)
        return std::nullopt;

    std::string_view path =
        node.filepath() ? std::string_view{*node.filepath()} : "<unknown>";

    auto label =
        fmt::format("{} [{}] {}", node.node_label(), node.source_range(), path);

    return Frame_Guard{bt, std::move(label)};
}

} // namespace frst::ast

#endif
