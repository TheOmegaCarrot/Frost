#ifndef FROST_AST_STATEMENT_HPP
#define FROST_AST_STATEMENT_HPP

#include <frost/ast/ast-node.hpp>
#include <frost/execution-context.hpp>

namespace frst::ast
{

//! @brief AST node that can be executed as a statement
class Statement : public AST_Node
{
  public:
    using Ptr = std::unique_ptr<Statement>;

    Statement(const Source_Range& source_range)
        : AST_Node(source_range)
    {
    }

    Statement() = delete;
    Statement(const Statement&) = delete;
    Statement(Statement&&) = delete;
    Statement& operator=(const Statement&) = delete;
    Statement& operator=(Statement&&) = delete;
    ~Statement() override = default;

    //! @brief Execute this statement
    void execute(Execution_Context ctx) const
    {
        do_execute(ctx);
    }

  protected:
    virtual void do_execute(Execution_Context& ctx) const = 0;
};

} // namespace frst::ast

#endif
