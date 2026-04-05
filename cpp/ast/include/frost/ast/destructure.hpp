#ifndef FROST_AST_DESTRUCTURE_HPP
#define FROST_AST_DESTRUCTURE_HPP

#include <frost/ast/ast-node.hpp>
#include <frost/backtrace.hpp>
#include <frost/execution-context.hpp>

namespace frst::ast
{

class Destructure : public AST_Node
{
  public:
    using Ptr = std::unique_ptr<Destructure>;

    Destructure(const Source_Range& source_range)
        : AST_Node(source_range)
    {
    }

    Destructure() = delete;
    Destructure(const Destructure&) = delete;
    Destructure(Destructure&&) = delete;
    Destructure& operator=(const Destructure&) = delete;
    Destructure& operator=(Destructure&&) = delete;
    ~Destructure() override = default;

    void destructure(Execution_Context ctx, const Value_Ptr& value) const
    {
        auto* bt = Backtrace_State::current();
        if (not bt)
            return do_destructure(ctx, value);

        Frame_Guard guard{bt,
                          fmt::format("{} [{}]", node_label(), source_range())};
        do_destructure(ctx, value);
    }

  protected:
    virtual void do_destructure(Execution_Context ctx,
                                const Value_Ptr& value) const = 0;
};

} // namespace frst::ast

#endif
