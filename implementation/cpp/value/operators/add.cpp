#include <frost/value.hpp>

#include "operators-common.hpp"

#include <ranges>

namespace frst
{
namespace
{

Map merge_maps(const Map& lhs, const Map& rhs)
{
    Map::key_container_type keys;
    Map::mapped_container_type values;

    keys.reserve(lhs.size() + rhs.size());
    values.reserve(lhs.size() + rhs.size());

    const auto push_pair = [&](const auto& entry) {
        keys.push_back(entry.first);
        values.push_back(entry.second);
    };

    auto lhs_itr = lhs.begin();
    auto rhs_itr = rhs.begin();
    const auto lhs_end = lhs.end();
    const auto rhs_end = rhs.end();

    const auto key_less = lhs.key_comp();

    while (lhs_itr != lhs_end && rhs_itr != rhs_end)
    {
        // Merge in elements one at a time, making sure to keep the key/value
        // containers sorted
        if (key_less(lhs_itr->first, rhs_itr->first))
        {
            push_pair(*lhs_itr);
            ++lhs_itr;
        }
        else if (key_less(rhs_itr->first, lhs_itr->first))
        {
            push_pair(*rhs_itr);
            ++rhs_itr;
        }
        else
        {
            // Keep rhs value on key collision
            push_pair(*rhs_itr);
            ++lhs_itr;
            ++rhs_itr;
        }
    }

    // After hitting the end of _one_, we gather up the rest
    // (one (or both) of these for loops will do nothing)
    for (; lhs_itr != lhs_end; ++lhs_itr)
        push_pair(*lhs_itr);

    for (; rhs_itr != rhs_end; ++rhs_itr)
        push_pair(*rhs_itr);

    return Map{std::sorted_unique, std::move(keys), std::move(values)};
}

} // namespace

[[noreturn]] void add_err(std::string_view lhs_type, std::string_view rhs_type)
{
    op_err("add", "+", lhs_type, rhs_type);
}

struct Add_Impl
{
    static Value operator()(const Frost_Numeric auto& lhs,
                            const Frost_Numeric auto& rhs)
    {
        return lhs + rhs;
    }
    static Value operator()(const String& lhs, const String& rhs)
    {
        return lhs + rhs;
    }
    static Value operator()(const Array& lhs, const Array& rhs)
    {
        return std::views::concat(lhs, rhs) | std::ranges::to<Array>();
    }
    static Value operator()(const Map& lhs, const Map& rhs)
    {
        return {Value::trusted, merge_maps(lhs, rhs)};
    }
    template <typename T, typename U>
    static Value operator()(const T&, const U&)
    {
        add_err(type_str<T>(), type_str<U>());
    }
} constexpr static add_impl;

Value_Ptr Value::add(const Value_Ptr& lhs, const Value_Ptr& rhs)
{
    return Value::create(std::visit(add_impl, lhs->value_, rhs->value_));
}

} // namespace frst
