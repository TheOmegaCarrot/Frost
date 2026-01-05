#ifndef FROST_AST_MOCK_EXPRESSION_HPP
#define FROST_AST_MOCK_EXPRESSION_HPP

#include <trompeloeil.hpp>

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

namespace frst::ast::mock
{

class Mock_Expression : public Expression
{
  public:
    MAKE_CONST_MOCK(evaluate, auto(const Symbol_Table&)->Value_Ptr, override);

  protected:
    std::string node_label() const override
    {
        return "Mock_Expression";
    }
};

} // namespace frst::ast::mock

#endif
