#ifndef FROST_AST_MATCH_HPP
#define FROST_AST_MATCH_HPP

#include <frost/ast/expression.hpp>
#include <frost/ast/match-pattern.hpp>
#include <frost/ast/utils/block-utils.hpp>

#include <flat_set>

namespace frst::ast
{

class Match final : public Expression
{
  public:
    using Ptr = std::unique_ptr<Match>;
    struct Arm
    {
        Match_Pattern::Ptr pattern;
        std::optional<Expression::Ptr> guard;
        Expression::Ptr result;
    };

    Match() = delete;
    Match(const Match&) = delete;
    Match(Match&&) = delete;
    Match& operator=(const Match&) = delete;
    Match& operator=(Match&&) = delete;
    ~Match() override = default;

    Match(const Source_Range& source_range, Expression::Ptr target,
          std::vector<Arm> arms)
        : Expression(source_range)
        , target_{std::move(target)}
        , arms_{std::move(arms)}
    {
    }

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        const auto no_symbols = [] -> std::generator<Symbol_Action> {
            co_return;
        };

        const auto get_name = [](const auto& action) -> const auto& {
            return action.name;
        };

        co_yield std::ranges::elements_of(target_->symbol_sequence());

        for (const auto& [pat, guard, result] : arms_)
        {
            std::flat_set<std::string> defns;

            for (const auto& action :
                 std::views::concat(pat->symbol_sequence(),
                                    guard.transform(utils::node_to_sym_seq)
                                        .value_or(no_symbols()),
                                    result->symbol_sequence()))
            {
                const auto name = action.visit(get_name);
                if (std::holds_alternative<Definition>(action))
                    defns.insert(name);
                else if (not defns.contains(name))
                    co_yield action;
            }
        }
    }

    std::generator<Child_Info> children() const final
    {
        co_yield make_child(target_, "Match_Target");

        for (const auto& [i, arm] : std::views::enumerate(arms_))
        {
            const auto& [pat, guard, result] = arm;

            co_yield make_child(pat, fmt::format("Pattern {}", i + 1));

            if (guard)
                co_yield make_child(guard.value(),
                                    fmt::format("Guard {}", i + 1));

            co_yield make_child(result, fmt::format("Result {}", i + 1));
        }
    }

  protected:
    Value_Ptr do_evaluate(Evaluation_Context ctx) const final
    {
        auto target = target_->evaluate(ctx);

        for (const auto& [pat, guard, result] : arms_)
        {
            Symbol_Table arm_table{&ctx.symbols};
            Execution_Context arm_ctx{.symbols = arm_table};

            // pat assigns into the arm_table
            if (not pat->try_match(arm_ctx, target))
                continue;

            if (guard
                && not guard.value()->evaluate(arm_ctx.as_eval())->truthy())
                continue;

            return result->evaluate(arm_ctx.as_eval());
        }

        throw Frost_Recoverable_Error{"Match expression found no match"};
    }

    std::string do_node_label() const final
    {
        return "Match";
    }

  private:
    Expression::Ptr target_;
    std::vector<Arm> arms_;
};

} // namespace frst::ast

#endif
