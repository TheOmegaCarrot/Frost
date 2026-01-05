#ifndef FROST_AST_MOCK_EXPRESSION_HPP
#define FROST_AST_MOCK_EXPRESSION_HPP

#include <trompeloeil.hpp>

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

namespace frost::ast::mock
{

class Mock_Expression : public frst::ast::Expression
{
  public:
    MAKE_CONST_MOCK(evaluate, frst::Value_Ptr(const frst::Symbol_Table&),
                    override);

  protected:
    std::string node_label() const override
    {
        return "Mock_Expression";
    }
};

} // namespace frost::ast::mock

#endif
