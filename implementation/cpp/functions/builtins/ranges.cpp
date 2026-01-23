#include "builtins-common.hpp"

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <algorithm>
#include <ranges>

namespace frst
{

constexpr static auto array_array =
    std::views::transform([](const std::ranges::range auto& range) {
        return Value::create(Array{std::from_range, range});
    });

#define ARR auto arr = args.at(0)->raw_get<Array>()

#define ARR_NUM                                                                \
    ARR;                                                                       \
    auto num = args.at(1)->raw_get<Int>()

#define GT0_NUM(NAME)                                                          \
    if (num <= 0)                                                              \
    throw Frost_Recoverable_Error{"Function " #NAME                            \
                                  " requires its numeric argument to be >0"}

#define GE0_NUM(NAME)                                                          \
    if (num < 0)                                                               \
    throw Frost_Recoverable_Error{"Function " #NAME                            \
                                  " requires its numeric argument to be >=0"}

#define DIRECT_NUM_IMPL(NAME)                                                  \
    return Value::create(arr | std::views::NAME(num) | std::ranges::to<Array>())

Value_Ptr stride(builtin_args_t args)
{
    REQUIRE_ARGS("stride", TYPES(Array), TYPES(Int));

    ARR_NUM;
    GT0_NUM(stride);

    DIRECT_NUM_IMPL(stride);
}

Value_Ptr take(builtin_args_t args)
{
    REQUIRE_ARGS("take", TYPES(Array), TYPES(Int));

    ARR_NUM;
    GE0_NUM(take);

    DIRECT_NUM_IMPL(take);
}

Value_Ptr drop(builtin_args_t args)
{
    REQUIRE_ARGS("drop", TYPES(Array), TYPES(Int));

    ARR_NUM;
    GE0_NUM(take);

    DIRECT_NUM_IMPL(drop);
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

#define DIRECT_NONE_IMPL(NAME)                                                 \
    return Value::create(arr | std::views::NAME | std::ranges::to<Array>());

Value_Ptr reverse(builtin_args_t args)
{
    REQUIRE_ARGS("reverse", TYPES(Array));

    ARR;

    DIRECT_NONE_IMPL(reverse);
}

Value_Ptr zip(builtin_args_t args)
{
    for (const auto& [i, arg] : std::views::enumerate(args))
        if (not arg->is<Array>())
            throw Frost_Recoverable_Error{
                fmt::format("Function zip requires Array for all arguments, "
                            "got {} for argument {}",
                            arg->type_name(), i)};

    std::vector<const Array*> raw_args =
        args
        | std::views::transform([](const Value_Ptr& arg) {
              return &arg->raw_get<Array>();
          })
        | std::ranges::to<std::vector>();

    const std::size_t smallest_len =
        (*std::ranges::min_element(raw_args, {}, [](const auto* val) {
            return val->size();
        }))->size();

    std::vector<Value_Ptr> result;
    result.reserve(smallest_len);

    for (auto i = 0uz; i < smallest_len; ++i)
    {
        Array row;
        row.reserve(raw_args.size());
        for (const auto& arr : raw_args)
            row.push_back(arr->at(i));
        result.push_back(Value::create(std::move(row)));
    }

    return Value::create(std::move(result));
}

void inject_ranges(Symbol_Table& table)
{
    INJECT(stride, 2, 2);
    INJECT(take, 2, 2);
    INJECT(drop, 2, 2);
    INJECT(slide, 2, 2);
    INJECT(chunk, 2, 2);
    INJECT(reverse, 1, 1);
    INJECT_V(zip, 2);
}
} // namespace frst
