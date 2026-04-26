#ifndef FROST_AST_MATCH_ALTERNATIVE_HPP
#define FROST_AST_MATCH_ALTERNATIVE_HPP

#include <frost/ast/match-pattern.hpp>

namespace frst::ast
{

class Match_Alternative final : public Match_Pattern
{
  public:
    using Ptr = std::unique_ptr<Match_Alternative>;

    Match_Alternative(const Source_Range& source_range,
                      std::vector<Match_Pattern::Ptr> alternatives);

    Match_Alternative() = delete;
    Match_Alternative(const Match_Alternative&) = delete;
    Match_Alternative(Match_Alternative&&) = delete;
    Match_Alternative& operator=(const Match_Alternative&) = delete;
    Match_Alternative& operator=(Match_Alternative&&) = delete;
    ~Match_Alternative() final = default;

    std::generator<Symbol_Action> symbol_sequence() const final;
    std::generator<Child_Info> children() const final;

  protected:
    bool do_try_match(Execution_Context ctx,
                      const Value_Ptr& value) const final;

    std::string do_node_label() const final
    {
        return "Match_Alternative";
    }

  private:
    std::vector<Match_Pattern::Ptr> alternatives_;
};

} // namespace frst::ast

#endif
