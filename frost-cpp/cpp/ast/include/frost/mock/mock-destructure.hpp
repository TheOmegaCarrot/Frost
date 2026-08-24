#ifndef FROST_AST_MOCK_DESTRUCTURE_HPP
#define FROST_AST_MOCK_DESTRUCTURE_HPP

#include <trompeloeil.hpp>

#include <frost/ast/destructure.hpp>
#include <frost/execution-context.hpp>
#include <frost/value.hpp>

namespace frst::mock
{

class Mock_Destructure : public ast::Destructure
{
  public:
    MAKE_CONST_MOCK(do_destructure,
                    auto(Execution_Context, const Value_Ptr&)->void, override);

    using Ptr = std::unique_ptr<Mock_Destructure>;

    Mock_Destructure()
        : Destructure(no_range)
    {
    }

    static Ptr make()
    {
        return std::make_unique<Mock_Destructure>();
    }

  protected:
    std::string do_node_label() const override
    {
        return "Mock_Destructure";
    }
};

} // namespace frst::mock

#endif
