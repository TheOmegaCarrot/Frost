#ifndef FROST_AST_MATCH_VALUE_HPP
#define FROST_AST_MATCH_VALUE_HPP

#include <frost/ast/expression.hpp>
#include <frost/ast/match-pattern.hpp>

namespace frst::ast
{

class Match_Value final : public Match_Pattern
{
  public:
    using Ptr = std::unique_ptr<Match_Value>;

    Match_Value(const Source_Range& source_range, Expression::Ptr expr)
        : Match_Pattern(source_range)
        , expr_{std::move(expr)}
    {
    }

    Match_Value() = delete;
    Match_Value(const Match_Value&) = delete;
    Match_Value(Match_Value&&) = delete;
    Match_Value& operator=(const Match_Value&) = delete;
    Match_Value& operator=(Match_Value&&) = delete;
    ~Match_Value() final = default;

    std::generator<Child_Info> children() const final;

  protected:
    bool do_try_match(Execution_Context ctx,
                      const Value_Ptr& value) const final;
    std::string do_node_label() const final;

  private:
    Expression::Ptr expr_;
};

} // namespace frst::ast

#endif
