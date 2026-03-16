#ifndef FROST_AST_EXPRESSION_HPP
#define FROST_AST_EXPRESSION_HPP

#include "statement.hpp"

#include <frost/backtrace.hpp>
#include <frost/value.hpp>

namespace frst::ast
{
class Expression : public Statement
{
  public:
    using Ptr = std::unique_ptr<Expression>;

    Expression(Source_Range source_range)
        : Statement(source_range)
    {
    }

    Expression() = delete;
    Expression(const Expression&) = delete;
    Expression(Expression&&) = delete;
    Expression& operator=(const Expression&) = delete;
    Expression& operator=(Expression&&) = delete;
    ~Expression() override = default;

    //! @brief Evaluate the expression, and get the value it evaluates to
    [[nodiscard]] Value_Ptr evaluate(Evaluation_Context ctx) const
    {
        if (not ctx.runtime.backtrace)
            return do_evaluate(ctx);

        Frame_Guard guard{ctx.runtime.backtrace, AST_Frame{.node = this}};

        try
        {
            return do_evaluate(ctx);
        }
        catch (Frost_Error& fe)
        {
            if (!fe.has_backtrace())
            {
                ctx.runtime.backtrace->snapshot_if_needed();
                fe.add_backtrace(
                    resolve_snapshot(ctx.runtime.backtrace->take_raw_snapshot()));
            }
            throw;
        }
    }

  protected:
    [[nodiscard]] virtual Value_Ptr do_evaluate(
        Evaluation_Context ctx) const = 0;

    //! Executing an expression is simply to evaluate it and discard the result
    std::optional<Map> do_execute(Execution_Context& ctx) const final
    {
        (void)evaluate(ctx.as_eval());
        return std::nullopt;
    }

  private:
    static std::unique_ptr<Backtrace> resolve_snapshot(
        std::vector<Backtrace_Frame> raw)
    {
        if (raw.empty())
            return nullptr;

        std::vector<Snapshot_Frame> resolved;
        resolved.reserve(raw.size());

        for (const auto& frame : raw)
        {
            std::visit(
                Overload{
                    [&](const AST_Frame& f) {
                        resolved.emplace_back(Resolved_AST_Frame{
                            .node_label = f.node->node_label(),
                            .source_range =
                                fmt::format("{}", f.node->source_range())});
                    },
                    [&](const Call_Frame& f) {
                        resolved.emplace_back(f);
                    },
                    [&](const Import_Frame& f) {
                        resolved.emplace_back(f);
                    },
                    [&](const Iterative_Frame& f) {
                        resolved.emplace_back(f);
                    },
                },
                frame);
        }

        return std::make_unique<Backtrace>(std::move(resolved));
    }
};
} // namespace frst::ast

#endif
