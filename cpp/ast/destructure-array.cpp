#include <frost/ast/destructure-array.hpp>

namespace frst::ast
{

void Destructure_Array::do_destructure(Execution_Context ctx,
                                       const Value_Ptr& value) const
{
    if (not value->is<Array>())
        throw Frost_Recoverable_Error{fmt::format(
            "Destructure expected Array, got {}", value->type_name())};

    const Array& arr = value->raw_get<Array>();

    if (arr.size() < destructures_.size())
    {
        throw Frost_Recoverable_Error{
            fmt::format("Insufficient Array elements to destructure: "
                        "required {} but got {}",
                        destructures_.size(), arr.size())};
    }

    if (not rest_name_ && arr.size() > destructures_.size())
    {
        throw Frost_Recoverable_Error{
            fmt::format("Too many Array elements to destructure: required "
                        "{} but got {}",
                        destructures_.size(), arr.size())};
    }

    for (const auto& [child, val] : std::views::zip(destructures_, arr))
        child->destructure(ctx, val);

    // permit `..._` as rest to discard rest
    if (rest_name_ == "_")
    {
        return;
    }
    else if (rest_name_)
    {
        ctx.symbols.define(
            rest_name_.value(),
            Value::create(arr
                          | std::views::drop(destructures_.size())
                          | std::ranges::to<Array>()));
    }
}

} // namespace frst::ast
