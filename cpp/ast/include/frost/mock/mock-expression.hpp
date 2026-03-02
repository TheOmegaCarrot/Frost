#ifndef FROST_AST_MOCK_EXPRESSION_HPP
#define FROST_AST_MOCK_EXPRESSION_HPP

#include <trompeloeil.hpp>

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

namespace frst::mock
{

class Mock_Expression : public ast::Expression
{
  public:
    MAKE_CONST_MOCK(evaluate, auto(const Symbol_Table&)->Value_Ptr, override);

    using Ptr = std::unique_ptr<Mock_Expression>;

    static Ptr make()
    {
        return std::make_unique<Mock_Expression>();
    }

  protected:
    std::string node_label() const override
    {
        return "Mock_Expression";
    }
};

} // namespace frst::mock

#endif
