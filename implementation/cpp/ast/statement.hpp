#ifndef FROST_AST_STATEMENT_HPP
#define FROST_AST_STATEMENT_HPP

#include <generator>
#include <memory>
#include <ostream>
#include <string>
#include <string_view>

#include <frost/symbol-table.hpp>

namespace frst::ast
{

//! @brief Common base class of all AST nodes / statements
class Statement
{
  public:
    using Ptr = std::unique_ptr<Statement>;

    Statement() = default;
    Statement(const Statement&) = delete;
    Statement(Statement&&) = delete;
    Statement& operator=(const Statement&) = delete;
    Statement& operator=(Statement&&) = delete;
    virtual ~Statement() = default;

    //! @brief Execute this statement
    //! @param table Symbol table to use for lookup, and in which to define new
    //!                 names
    virtual void execute(Symbol_Table& table) const = 0;

    //! @brief Print AST of this node and all descendents
    void debug_dump_ast(std::ostream& out) const;

  protected:
    struct Child_Info
    {
        const Statement* node = nullptr;
        std::string_view label;
    };

    static Child_Info make_child(const auto& child, std::string_view label = {})
    {
        return Child_Info{child.get(), label};
    }

    virtual std::string node_label() const = 0;

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
};

} // namespace frst::ast

#endif
