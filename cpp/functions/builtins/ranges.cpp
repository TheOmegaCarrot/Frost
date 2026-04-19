#include <flat_map>
#include <frost/builtins-common.hpp>

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

namespace
{
void require_positive(std::string_view name, Int num)
{
    if (num <= 0)
        throw Frost_Recoverable_Error{fmt::format(
            "Function {} requires its numeric argument to be >0", name)};
}

void require_nonnegative(std::string_view name, Int num)
{
    if (num < 0)
        throw Frost_Recoverable_Error{fmt::format(
            "Function {} requires its numeric argument to be >=0", name)};
}

template <auto adaptor>
Value_Ptr apply_view(const Array& arr, Int num)
{
    return Value::create(arr | adaptor(num) | std::ranges::to<Array>());
}

template <auto adaptor>
Value_Ptr apply_rev_view(const Array& arr, Int num)
{
    return Value::create(arr
                         | std::views::reverse
                         | adaptor(num)
                         | std::views::reverse
                         | std::ranges::to<Array>());
}

template <auto adaptor>
Value_Ptr apply_rewrap_view(const Array& arr, Int num)
{
    return Value::create(
        arr | adaptor(num) | array_array | std::ranges::to<Array>());
}

template <auto adaptor>
Value_Ptr apply_pred_view(const Array& arr, const Function& fn)
{
    return Value::create(arr
                         | adaptor([&](const Value_Ptr& val) {
                               return fn->call({val})->truthy();
                           })
                         | std::ranges::to<Array>());
}

template <auto algorithm>
Value_Ptr quantifier_impl(std::string_view name, builtin_args_t args)
{
    REQUIRE_ARGS(name, TYPES(Array), OPTIONAL(TYPES(Function)));

    const auto& arr = GET(0, Array);

    if (HAS(1))
    {
        const auto& fn = GET(1, Function);
        return Value::create(algorithm(arr, [&](const Value_Ptr& elem) {
            return fn->call({elem})->truthy();
        }));
    }

    return Value::create(algorithm(arr, [](const Value_Ptr& elem) {
        return elem->truthy();
    }));
}
} // namespace

BUILTIN(stride)
{
    REQUIRE_ARGS("stride", TYPES(Array), TYPES(Int));

    const auto& arr = GET(0, Array);
    auto num = GET(1, Int);
    require_positive("stride", num);

    return apply_view<std::views::stride>(arr, num);
}

BUILTIN(take)
{
    REQUIRE_ARGS("take", TYPES(Array), TYPES(Int));

    const auto& arr = GET(0, Array);
    auto num = GET(1, Int);
    require_nonnegative("take", num);

    return apply_view<std::views::take>(arr, num);
}

BUILTIN(drop)
{
    REQUIRE_ARGS("drop", TYPES(Array), TYPES(Int));

    const auto& arr = GET(0, Array);
    auto num = GET(1, Int);
    require_nonnegative("drop", num);

    return apply_view<std::views::drop>(arr, num);
}

BUILTIN(tail)
{
    REQUIRE_ARGS("tail", TYPES(Array), TYPES(Int));

    const auto& arr = GET(0, Array);
    auto num = GET(1, Int);
    require_nonnegative("tail", num);

    return apply_rev_view<std::views::take>(arr, num);
}

BUILTIN(drop_tail)
{
    REQUIRE_ARGS("drop_tail", TYPES(Array), TYPES(Int));

    const auto& arr = GET(0, Array);
    auto num = GET(1, Int);
    require_nonnegative("drop_tail", num);

    return apply_rev_view<std::views::drop>(arr, num);
}

BUILTIN(slide)
{
    REQUIRE_ARGS("slide", TYPES(Array), TYPES(Int));

    const auto& arr = GET(0, Array);
    auto num = GET(1, Int);
    require_positive("slide", num);

    return apply_rewrap_view<std::views::slide>(arr, num);
}

BUILTIN(chunk)
{
    REQUIRE_ARGS("chunk", TYPES(Array), TYPES(Int));

    const auto& arr = GET(0, Array);
    auto num = GET(1, Int);
    require_positive("chunk", num);

    return apply_rewrap_view<std::views::chunk>(arr, num);
}

BUILTIN(reverse)
{
    REQUIRE_ARGS("reverse", TYPES(Array));

    const auto& arr = GET(0, Array);

    return Value::create(arr | std::views::reverse | std::ranges::to<Array>());
}

// TODO: {take,drop}_while can evaluate the predicate multiple times
//       per element
BUILTIN(take_while)
{
    REQUIRE_ARGS("take_while", TYPES(Array), TYPES(Function));

    const auto& arr = GET(0, Array);
    const auto& fn = GET(1, Function);

    return apply_pred_view<std::views::take_while>(arr, fn);
}

BUILTIN(drop_while)
{
    REQUIRE_ARGS("drop_while", TYPES(Array), TYPES(Function));

    const auto& arr = GET(0, Array);
    const auto& fn = GET(1, Function);

    return apply_pred_view<std::views::drop_while>(arr, fn);
}

BUILTIN(chunk_by)
{
    REQUIRE_ARGS("chunk_by", TYPES(Array), TYPES(Function));

    const auto& arr = GET(0, Array);
    const auto& fn = GET(1, Function);

    return Value::create(arr
                         | std::views::chunk_by([&](const Value_Ptr& first,
                                                    const Value_Ptr& second) {
                               return fn->call({first, second})->truthy();
                           })
                         | array_array
                         | std::ranges::to<Array>());
}

// Shared implementation for zip and zip_with. Iterates corresponding
// elements from each array (truncating to the shortest) and calls
// `combine` with a span of the current row. Raw pointers to the inner
// Arrays avoid copying the flat_map containers out of their Value shells.
Array zip_impl(std::span<const Value_Ptr> arrays, auto&& combine)
{
    std::vector<const Array*> cols =
        arrays
        | std::views::transform([](const Value_Ptr& arg) {
              return &arg->raw_get<Array>();
          })
        | std::ranges::to<std::vector>();

    const std::size_t shortest =
        (*std::ranges::min_element(cols, {}, [](const auto* a) {
            return a->size();
        }))->size();

    auto gather_row = [&](std::size_t i) {
        return cols
               | std::views::transform([i](const Array* a) {
                     return a->at(i);
                 })
               | std::ranges::to<std::vector>();
    };

    return std::views::iota(0uz, shortest)
           | std::views::transform([&](std::size_t i) {
                 auto row = gather_row(i);
                 return combine(std::span<const Value_Ptr>{row});
             })
           | std::ranges::to<Array>();
}

BUILTIN(zip)
{
    REQUIRE_ARITY("zip", 2);
    UNIFORM_VARIADIC(zip, Array);

    return Value::create(zip_impl(args, [](std::span<const Value_Ptr> row) {
        return Value::create(Array{std::from_range, row});
    }));
}

BUILTIN(zip_with)
{
    REQUIRE_ARGS("zip_with", PARAM("function", TYPES(Function)),
                 VARIADIC_REST(2, "array", TYPES(Array)));

    const auto& fn = GET(0, Function);
    auto arrays = args.subspan(1);

    return Value::create(zip_impl(arrays, [&](std::span<const Value_Ptr> row) {
        return fn->call(row);
    }));
}

// Shared implementation for xprod and xprod_with. Iterates the cartesian
// product of all input arrays and calls `combine` with a span of the
// current tuple.
Array xprod_impl(std::span<const Value_Ptr> arrays, auto&& combine)
{
    std::vector<const Array*> raw_args =
        arrays
        | std::views::transform([](const Value_Ptr& arg) {
              return &arg->raw_get<Array>();
          })
        | std::ranges::to<std::vector>();

    for (const auto* arr : raw_args)
        if (arr->empty())
            return {};

    // like a mixed-radix odometer
    struct Tracked_Index
    {
        std::size_t index = 0;
        std::size_t size;
    };
    std::vector<Tracked_Index> tracked_indices;
    tracked_indices.reserve(arrays.size());
    for (const auto* arr : raw_args)
        tracked_indices.push_back({.size = arr->size()});

    Array result;

    while (true)
    {
        // gather the current tuple according to tracked indices
        std::vector<Value_Ptr> row;
        row.reserve(raw_args.size());
        for (const auto& [raw_arg, idx_tracker] :
             std::views::zip(raw_args, tracked_indices))
            row.push_back(raw_arg->at(idx_tracker.index));

        result.push_back(combine(std::span<const Value_Ptr>{row}));

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

    return result;
}

BUILTIN(xprod)
{
    REQUIRE_ARITY("xprod", 2);
    UNIFORM_VARIADIC(xprod, Array);

    return Value::create(xprod_impl(args, [](std::span<const Value_Ptr> row) {
        return Value::create(Array{std::from_range, row});
    }));
}

BUILTIN(xprod_with)
{
    REQUIRE_ARGS("xprod_with", PARAM("function", TYPES(Function)),
                 VARIADIC_REST(2, "array", TYPES(Array)));

    const auto& fn = GET(0, Function);
    auto arrays = args.subspan(1);

    return Value::create(
        xprod_impl(arrays, [&](std::span<const Value_Ptr> row) {
            return fn->call(row);
        }));
}

BUILTIN(transform)
{
    REQUIRE_ARGS("transform", PARAM("structure", TYPES(Array, Map)),
                 TYPES(Function));

    return Value::do_map(args.at(0), GET(1, Function), "Builtin transform");
}

BUILTIN(select)
{
    REQUIRE_ARGS("select", PARAM("structure", TYPES(Array, Map)),
                 TYPES(Function));

    return Value::do_filter(args.at(0), GET(1, Function));
}

BUILTIN(fold)
{
    REQUIRE_ARGS("fold", PARAM("structure", TYPES(Array, Map)), TYPES(Function),
                 OPTIONAL(PARAM("init", ANY)));

    return Value::do_reduce(args.at(0), GET(1, Function),
                            [&] -> std::optional<Value_Ptr> {
                                if (args.size() == 3)
                                    return args.at(2);
                                else
                                    return std::nullopt;
                            }());
}

BUILTIN(sorted)
{
    REQUIRE_ARGS("sorted", TYPES(Array), OPTIONAL(TYPES(Function)));

    Function sorter = [&] {
        if (HAS(1))
        {
            return GET(1, Function);
        }
        else
        {
            return system_function([](builtin_args_t args) {
                REQUIRE_ARGS("sorted.<comparator>", ANY, ANY);
                return Value::less_than(args.at(0), args.at(1));
            });
        }
    }();

    const auto& in = GET(0, Array);
    Array out{std::from_range, in};

    std::ranges::stable_sort(out, [&](const Value_Ptr& l, const Value_Ptr& r) {
        return sorter->call({l, r})->truthy();
    });

    return Value::create(std::move(out));
}

BUILTIN(sort_by)
{
    REQUIRE_ARGS("sort_by", TYPES(Array), PARAM("projection", TYPES(Function)));

    const auto& proj = GET(1, Function);

    auto projected = GET(0, Array)
                     | std::views::transform([&](const Value_Ptr& val) {
                           return std::pair{val, proj->call({val})};
                       })
                     | std::ranges::to<std::vector>();

    std::ranges::stable_sort(
        projected,
        [](const Value_Ptr& l, const Value_Ptr& r) {
            return Value::less_than(l, r)->truthy();
        },
        [](const auto& pair) {
            return pair.second;
        });

    return Value::create(
        projected | std::views::keys | std::ranges::to<Array>());
}

BUILTIN(any)
{
    return quantifier_impl<std::ranges::any_of>("any", args);
}
BUILTIN(all)
{
    return quantifier_impl<std::ranges::all_of>("all", args);
}
BUILTIN(none)
{
    return quantifier_impl<std::ranges::none_of>("none", args);
}

BUILTIN(repeat)
{
    REQUIRE_ARGS("repeat", ANY, TYPES(Int));

    auto num = GET(1, Int);
    require_nonnegative("repeat", num);

    return Value::create(std::views::repeat(args.at(0), num)
                         | std::ranges::to<Array>());
}

namespace
{
template <typename T>
auto make_map_generalizer_for(const T&)
{
    return [](T::value_type&& kv) {
        return std::pair<const Value_Ptr, Value_Ptr>{
            std::move(kv.first), Value::create(std::move(kv.second))};
    };
}

template <typename T>
Value_Ptr generalize_map(T&& map)
{
    static_assert(not std::is_lvalue_reference_v<T>);
    static_assert(std::is_rvalue_reference_v<decltype(map)>);

    return Value::create(std::move(map)
                         | std::views::transform(make_map_generalizer_for(map))
                         | std::ranges::to<Map>());
}
} // namespace

BUILTIN(group_by)
{
    REQUIRE_ARGS("group_by", TYPES(Array), TYPES(Function));

    std::flat_map<Value_Ptr, Array, impl::Value_Ptr_Less> groups;

    auto arr = GET(0, Array);
    auto fn = GET(1, Function);

    for (const auto& elem : arr)
    {
        auto key = fn->call({elem});

        auto itr = groups.find(key);
        if (itr == groups.end())
        {
            groups.emplace(key, Array{elem});
        }
        else
        {
            itr->second.push_back(elem);
        }
    }

    return generalize_map(std::move(groups));
}

BUILTIN(count_by)
{
    REQUIRE_ARGS("count_by", TYPES(Array), TYPES(Function));

    std::flat_map<Value_Ptr, Int, impl::Value_Ptr_Less> counts;

    auto arr = GET(0, Array);
    auto fn = GET(1, Function);

    for (const auto& elem : arr)
    {
        auto key = fn->call({elem});

        ++counts[key]; // leverage operator[] default-constructing
    }

    return generalize_map(std::move(counts));
}

BUILTIN(find)
{
    REQUIRE_ARGS("find", TYPES(Array), PARAM("predicate", TYPES(Function)));

    const auto& arr = GET(0, Array);
    const auto& pred = GET(1, Function);
    auto itr = std::ranges::find_if(arr, [&](const Value_Ptr& val) {
        return pred->call({val})->truthy();
    });

    if (itr == arr.end())
        return Value::null();

    return *itr;
}

BUILTIN(scan)
{
    REQUIRE_ARGS("scan", TYPES(Array), TYPES(Function));

    auto arr = GET(0, Array);

    if (arr.empty())
        return Value::create(Array{});

    auto fn = GET(1, Function);

    Value_Ptr acc = arr.at(0);

    Array result{acc};
    result.reserve(arr.size() + 1);

    for (const auto& elem : std::views::drop(arr, 1))
    {
        acc = fn->call({acc, elem});
        result.push_back(acc);
    }

    return Value::create(std::move(result));
}

namespace
{
Array do_flatten(const Array& arr)
{
    Array out;
    for (const auto& elem : arr)
    {
        if (elem->is<Array>())
            std::ranges::move(do_flatten(elem->raw_get<Array>()),
                              std::back_inserter(out));
        else
            out.push_back(elem);
    }
    return out;
}

Array do_flatten_n(const Array& arr, Int depth)
{
    Array out;
    for (const auto& elem : arr)
    {
        if (depth > 0 && elem->is<Array>())
            std::ranges::move(do_flatten_n(elem->raw_get<Array>(), depth - 1),
                              std::back_inserter(out));
        else
            out.push_back(elem);
    }
    return out;
}
} // namespace

BUILTIN(flatten)
{
    REQUIRE_ARGS("flatten", TYPES(Array), OPTIONAL(PARAM("n", TYPES(Int))));

    const auto& arr = GET(0, Array);

    if (HAS(1))
    {
        auto num = GET(1, Int);
        require_nonnegative("flatten", num);
        return Value::create(do_flatten_n(arr, num));
    }

    return Value::create(do_flatten(arr));
}

BUILTIN(partition)
{
    REQUIRE_ARGS("partition", TYPES(Array), TYPES(Function));

    auto arr = GET(0, Array);
    auto fn = GET(1, Function);

    Array pass;
    Array fail;

    STRINGS(pass, fail);

    for (const auto& elem : arr)
    {
        (fn->call({elem})->truthy() ? pass : fail).push_back(elem);
    }

    return Value::create(Value::trusted,
                         Map{
                             {strings.pass, Value::create(std::move(pass))},
                             {strings.fail, Value::create(std::move(fail))},
                         });
}

BUILTIN(slice)
{
    REQUIRE_ARGS("slice", PARAM("arr", TYPES(Array)),
                 PARAM("start", TYPES(Int)),
                 OPTIONAL(PARAM("end", TYPES(Int))));

    const auto& arr = GET(0, Array);
    auto len = static_cast<Int>(arr.size());

    auto clamp = [len](Int idx) -> std::size_t {
        if (idx < 0)
            idx += len;
        return static_cast<std::size_t>(std::clamp(idx, Int{0}, len));
    };

    auto start = clamp(GET(1, Int));
    auto end = HAS(2) ? clamp(GET(2, Int)) : static_cast<std::size_t>(len);

    if (start >= end)
        return Value::create(Array{});

    return Value::create(Array{arr.begin() + static_cast<Int>(start),
                               arr.begin() + static_cast<Int>(end)});
}

void inject_ranges(Symbol_Table& table)
{
    INJECT(slice);
    INJECT(stride);
    INJECT(take);
    INJECT(drop);
    INJECT(tail);
    INJECT(drop_tail);
    INJECT(slide);
    INJECT(chunk);
    INJECT(reverse);
    INJECT(take_while);
    INJECT(drop_while);
    INJECT(chunk_by);
    INJECT(zip);
    INJECT(zip_with);
    INJECT(xprod);
    INJECT(xprod_with);
    INJECT(transform);
    INJECT(select);
    INJECT(fold);
    INJECT(sorted);
    INJECT(sort_by);
    INJECT(any);
    INJECT(all);
    INJECT(none);
    INJECT(repeat);
    INJECT(group_by);
    INJECT(count_by);
    INJECT(find);
    INJECT(scan);
    INJECT(flatten);
    INJECT(partition);
}
} // namespace frst
