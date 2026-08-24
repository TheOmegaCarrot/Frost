#ifndef FROST_AST_MOCK_MATCH_PATTERN_HPP
#define FROST_AST_MOCK_MATCH_PATTERN_HPP

#include <trompeloeil.hpp>

#include <frost/ast/match-pattern.hpp>
#include <frost/execution-context.hpp>
#include <frost/value.hpp>

namespace frst::mock
{

class Mock_Match_Pattern : public ast::Match_Pattern
{
  public:
    MAKE_CONST_MOCK(do_try_match,
                    auto(Execution_Context, const Value_Ptr&)->bool, override);

    using Ptr = std::unique_ptr<Mock_Match_Pattern>;

    Mock_Match_Pattern()
        : Match_Pattern(no_range)
    {
    }

    static Ptr make()
    {
        return std::make_unique<Mock_Match_Pattern>();
    }

  protected:
    std::string do_node_label() const override
    {
        return "Mock_Match_Pattern";
    }
};

} // namespace frst::mock

#endif
