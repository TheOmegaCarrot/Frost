#ifndef FROST_AST_MOCK_CALLABLE_HPP
#define FROST_AST_MOCK_CALLABLE_HPP

#include <trompeloeil.hpp>

#include <frost/value.hpp>

namespace frst::mock
{

class Mock_Callable : public Callable
{
  public:
    MAKE_CONST_MOCK(call, auto(std::span<const Value_Ptr>)->Value_Ptr,
                    override);

    MAKE_CONST_MOCK(debug_dump, auto()->std::string, override);

    using Ptr = std::shared_ptr<Mock_Callable>;

    static Ptr make()
    {
        return std::make_shared<Mock_Callable>();
    }
};

} // namespace frst::mock

#endif
