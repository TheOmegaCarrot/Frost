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

#define ARR const auto& arr = args.at(0)->raw_get<Array>()

#define ARR_NUM                                                                \
    ARR;                                                                       \
    auto num = args.at(1)->raw_get<Int>()

#define ARR_FN                                                                 \
    ARR;                                                                       \
    const auto& fn = args.at(1)->raw_get<Function>()

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

#define PRED_IMPL(NAME)                                                        \
    return Value::create(arr                                                   \
                         | std::views::NAME([&](const Value_Ptr& val) {        \
                               return fn->call({val})->as<Bool>().value();     \
                           })                                                  \
                         | std::ranges::to<Array>());

// TODO: {take,drop}_while can evaluate the predicate multiple times
//       per element
Value_Ptr take_while(builtin_args_t args)
{
    REQUIRE_ARGS("take_while", TYPES(Array), TYPES(Function));

    ARR_FN;

    PRED_IMPL(take_while);
}

Value_Ptr drop_while(builtin_args_t args)
{
    REQUIRE_ARGS("drop_while", TYPES(Array), TYPES(Function));

    ARR_FN;

    PRED_IMPL(drop_while);
}

Value_Ptr chunk_by(builtin_args_t args)
{
    REQUIRE_ARGS("chunk_by", TYPES(Array), TYPES(Function));

    ARR_FN;

    return Value::create(
        arr
        | std::views::chunk_by(
            [&](const Value_Ptr& first, const Value_Ptr& second) {
                return fn->call({first, second})->as<Bool>().value();
            })
        | array_array
        | std::ranges::to<Array>());
    ;
}

Value_Ptr zip(builtin_args_t args)
{
    UNIFORM_VARIADIC(zip, Array);

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

Value_Ptr xprod(builtin_args_t args)
{
    UNIFORM_VARIADIC(xprod, Array);

    std::vector<const Array*> raw_args =
        args
        | std::views::transform([](const Value_Ptr& arg) {
              return &arg->raw_get<Array>();
          })
        | std::ranges::to<std::vector>();

    for (const auto& arr : raw_args)
        if (arr->empty())
            return Value::create(Array{});

    // like a mixed-radix odometer
    struct Tracked_Index
    {
        std::size_t index = 0;
        std::size_t size;
    };
    std::vector<Tracked_Index> tracked_indices;
    tracked_indices.reserve(args.size());
    for (const auto* arr : raw_args)
        tracked_indices.push_back({.size = arr->size()});

    Array result;

    while (true)
    {
        Array elem;
        elem.reserve(raw_args.size());

        // push in a row according to tracked indices
        for (const auto& [raw_arg, idx_tracker] :
             std::views::zip(raw_args, tracked_indices))
            elem.push_back(raw_arg->at(idx_tracker.index));

        result.push_back(Value::create(std::move(elem)));

        // bump up the tracked indices
        bool are_carrying = true;
        // walk backwards
        for (auto& tidx : std::views::reverse(tracked_indices))
        {
            if (not are_carrying) // if we didn't hit a max, we're done
                                  // incrementing
                break;

            if (++tidx.index < tidx.size) // increment, and if we hit the max...
                are_carrying = false;     // ...time to "carry" on (hah)
            else
                tidx.index = 0; // oh yeah and reset this one
        }

        // if we "carried" to the end, the whole thing would "roll over"
        // so we're done with this whole cartesian product operation
        if (are_carrying)
            break;
    }

    return Value::create(std::move(result));
}

Value_Ptr transform(builtin_args_t args)
{
    REQUIRE_ARGS("transform", PARAM("structure", TYPES(Array, Map)),
                 TYPES(Function));

    return Value::do_map(args.at(0), args.at(1)->raw_get<Function>(),
                         "Builtin transform");
}

Value_Ptr select(builtin_args_t args)
{
    REQUIRE_ARGS("select", PARAM("structure", TYPES(Array, Map)),
                 TYPES(Function));

    return Value::do_filter(args.at(0), args.at(1)->raw_get<Function>());
}

Value_Ptr fold(builtin_args_t args)
{
    REQUIRE_ARGS("fold", PARAM("structure", TYPES(Array, Map)), TYPES(Function),
                 OPTIONAL(PARAM("init", ANY)));

    return Value::do_reduce(args.at(0), args.at(1)->raw_get<Function>(),
                            [&] -> std::optional<Value_Ptr> {
                                if (args.size() == 3)
                                    return args.at(2);
                                else
                                    return std::nullopt;
                            }());
}

void inject_ranges(Symbol_Table& table)
{
    INJECT(stride, 2, 2);
    INJECT(take, 2, 2);
    INJECT(drop, 2, 2);
    INJECT(slide, 2, 2);
    INJECT(chunk, 2, 2);
    INJECT(reverse, 1, 1);
    INJECT(take_while, 2, 2);
    INJECT(drop_while, 2, 2);
    INJECT(chunk_by, 2, 2);
    INJECT_V(zip, 2);
    INJECT_V(xprod, 2);
    INJECT(transform, 2, 2);
    INJECT(select, 2, 2);
    INJECT(fold, 2, 3);
}
} // namespace frst
