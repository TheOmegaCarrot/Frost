#include "builtins-common.hpp"

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <ranges>

namespace frst
{

constexpr static auto array_array =
    std::views::transform([](const std::ranges::range auto& range) {
        return Value::create(Array{std::from_range, range});
    });

#define ARR_NUM                                                                \
    auto arr = args.at(0)->raw_get<Array>();                                   \
    auto num = args.at(1)->raw_get<Int>()

#define GT0_NUM(NAME)                                                          \
    if (num <= 0)                                                              \
    throw Frost_Recoverable_Error{"Function " #NAME                            \
                                  " requires its numeric argument to be >0"}

#define GE0_NUM(NAME)                                                          \
    if (num < 0)                                                               \
    throw Frost_Recoverable_Error{"Function " #NAME                            \
                                  " requires its numeric argument to be >=0"}

#define DIRECT_IMPL(NAME)                                                      \
    return Value::create(arr | std::views::NAME(num) | std::ranges::to<Array>())

Value_Ptr stride(builtin_args_t args)
{
    REQUIRE_ARGS("stride", TYPES(Array), TYPES(Int));

    ARR_NUM;
    GT0_NUM(stride);

    DIRECT_IMPL(stride);
}

Value_Ptr take(builtin_args_t args)
{
    REQUIRE_ARGS("take", TYPES(Array), TYPES(Int));

    ARR_NUM;
    GE0_NUM(take);

    DIRECT_IMPL(take);
}

Value_Ptr drop(builtin_args_t args)
{
    REQUIRE_ARGS("drop", TYPES(Array), TYPES(Int));

    ARR_NUM;
    GE0_NUM(take);

    DIRECT_IMPL(drop);
}

#define REWRAP_IMPL(NAME)                                                      \
    return Value::create(                                                      \
        arr | std::views::NAME(num) | array_array | std::ranges::to<Array>());

Value_Ptr slide(builtin_args_t args)
{
    REQUIRE_ARGS("slide", TYPES(Array), TYPES(Int));

    ARR_NUM;
    GT0_NUM(slide);

    REWRAP_IMPL(slide);
}

Value_Ptr chunk(builtin_args_t args)
{
    REQUIRE_ARGS("chunk", TYPES(Array), TYPES(Int));

    ARR_NUM;
    GT0_NUM(chunk);

    REWRAP_IMPL(chunk);
}

void inject_ranges(Symbol_Table& table)
{
    INJECT(stride, 2, 2);
    INJECT(take, 2, 2);
    INJECT(drop, 2, 2);
    INJECT(slide, 2, 2);
    INJECT(chunk, 2, 2);
}
} // namespace frst
