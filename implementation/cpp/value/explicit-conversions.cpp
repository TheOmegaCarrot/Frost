#include <frost/utils.hpp>
#include <frost/value.hpp>

#include <fmt/format.h>

#include <charconv>
#include <ranges>

using namespace std::literals;

namespace frst
{

struct To_String_Impl
{
    std::string operator()(
        const Null&,
        [[maybe_unused]] To_Internal_String_Params params = {}) const
    {
        return "null";
    }

    std::string operator()(
        const Int& integer,
        [[maybe_unused]] To_Internal_String_Params params = {}) const
    {
        char buf[24]{};
        auto [ptr, err] = std::to_chars(std::ranges::begin(buf),
                                        std::ranges::end(buf), integer);
        if (err != std::errc{})
            throw Frost_Internal_Error{fmt::format(
                "Int->String error: {}", std::make_error_code(err).message())};

        return buf;
    }

    std::string operator()(
        const Float& flt,
        [[maybe_unused]] To_Internal_String_Params params = {}) const
    {
        char buf[24]{};
        auto [ptr, err] =
            std::to_chars(std::ranges::begin(buf), std::ranges::end(buf), flt);
        if (err != std::errc{})
            throw Frost_Internal_Error{
                fmt::format("Float->String error: {}",
                            std::make_error_code(err).message())};

        return buf;
    }

    std::string operator()(const String& str,
                           To_Internal_String_Params params = {}) const
    {
        if (params.in_structure)
            return fmt::format("{:?}", str);
        else
            return str;
    }

    std::string operator()(
        const Bool& b,
        [[maybe_unused]] To_Internal_String_Params params = {}) const
    {
        return b ? "true" : "false";
    }

    std::string operator()(
        const Array& arr,
        [[maybe_unused]] To_Internal_String_Params params = {}) const
    {
        if (arr.empty())
            return "[]";

        params.in_structure = true;
        ++params.depth;

        std::string_view joiner = params.pretty ? ",\n"sv : ", ";

        auto array_insides =
            arr
            | std::views::transform([&](const auto& value) {
                  if (params.pretty)
                  {
                      return fmt::format("{: >{}}{}", "", params.depth * 4,
                                         value->to_internal_string(params));
                  }
                  return value->to_internal_string(params);
              })
            | std::views::join_with(joiner)
            | std::ranges::to<std::string>();

        if (params.pretty)
            return fmt::format("[\n{}\n{: >{}}]", array_insides, "",
                               (params.depth - 1) * 4);
        else
            return fmt::format("[ {} ]", array_insides);
    }

    std::string operator()(
        const Map& map,
        [[maybe_unused]] To_Internal_String_Params params = {}) const
    {
        if (map.empty())
            return "{}";

        params.in_structure = true;
        ++params.depth;

        std::string_view joiner = params.pretty ? ",\n"sv : ", ";

        auto map_insides =
            map
            | std::views::transform([&](const auto& kv) {
                  const auto& [k, v] = kv;
                  std::size_t indent = params.pretty ? params.depth * 4 : 0;

                  if (params.pretty
                      && k->template as<String>()
                             .transform([](const String& key) {
                                 return utils::is_identifier_like(key)
                                        && not utils::is_reserved_keyword(key);
                             })
                             .value_or(false))
                  {
                      return fmt::format(R"({: >{}}{}: {})", "", indent,
                                         k->to_internal_string(),
                                         v->to_internal_string(params));
                  }
                  else
                  {

                      return fmt::format(R"({: >{}}[{}]: {})", "", indent,
                                         k->to_internal_string(params),
                                         v->to_internal_string(params));
                  }
              })
            | std::views::join_with(joiner)
            | std::ranges::to<std::string>();

        if (params.pretty)
            return fmt::format("{{\n{}\n{: >{}}}}", map_insides, "",
                               (params.depth - 1) * 4);
        else
            return fmt::format("{{ {} }}", map_insides);
    }

    std::string operator()(
        const Function&,
        [[maybe_unused]] To_Internal_String_Params params = {}) const
    {
        return "<Function>";
    }

} constexpr static to_string_impl;

std::string Value::to_internal_string(To_Internal_String_Params params) const
{
    return value_.visit([&](const auto& value) {
        return to_string_impl(value, params);
    });
}

struct To_Int_Impl
{
    static std::optional<Int> operator()(const String& value)
    {
        Int result{};
        auto begin = value.data();
        auto end = begin + value.size();
        auto [ptr, err] = std::from_chars(begin, end, result);

        if (err != std::error_code{})
            return std::nullopt;

        if (ptr != end)
            return std::nullopt;

        return result;
    }

    static std::optional<Int> operator()(const auto&)
    {
        return std::nullopt;
    }
} constexpr static to_int_impl;

std::optional<Int> Value::to_internal_int() const
{
    return as<Int>().or_else([&] {
        return value_.visit(to_int_impl);
    });
}

Value_Ptr Value::to_int() const
{
    return to_internal_int()
        .transform([](const Int v) {
            return create(auto{v});
        })
        .value_or(Value::null());
}

struct To_Float_Impl
{
    static std::optional<Float> operator()(const String& value)
    {
        Float result{};
        auto begin = value.data();
        auto end = begin + value.size();
        auto [ptr, err] = std::from_chars(begin, end, result);

        if (err != std::error_code{})
            return std::nullopt;

        if (ptr != end)
            return std::nullopt;

        return result;
    }

    static std::optional<Float> operator()(const auto&)
    {
        return std::nullopt;
    }
} constexpr static to_float_impl;

std::optional<Float> Value::to_internal_float() const
{
    return as<Float>().or_else([&] {
        return value_.visit(to_float_impl);
    });
}

Value_Ptr Value::to_float() const
{
    return to_internal_float()
        .transform([](const Float v) {
            return create(auto{v});
        })
        .value_or(Value::null());
}

} // namespace frst
