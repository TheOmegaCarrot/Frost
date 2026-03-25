#include "frost/builtin.hpp"
#include "frost/value.hpp"
#include <frost/builtins-common.hpp>

#include <boost/json.hpp>

namespace frst
{

namespace json
{
struct Decode_Json_Impl
{

    Value_Ptr operator()(this const auto, const std::nullptr_t&)
    {
        return Value::null();
    }

    Value_Ptr operator()(this const auto, const bool& b)
    {
        return Value::create(auto{b});
    }

    Value_Ptr operator()(this const auto, const boost::json::string& str)
    {
        return Value::create(String{str});
    }

    Value_Ptr operator()(this const auto, const std::int64_t& i)
    {
        return Value::create(auto{i});
    }

    Value_Ptr operator()(this const auto, const double& d)
    {
        return Value::create(auto{d});
    }

    Value_Ptr operator()(this const auto, const std::uint64_t& u)
    {
        throw Frost_Recoverable_Error{
            fmt::format("Value {} is out of range", u)};
    }

    Value_Ptr operator()(this const auto self, const boost::json::array& arr)
    {
        return Value::create(arr
                             | std::views::transform([&](const auto& elem) {
                                   return boost::json::visit(self, elem);
                               })
                             | std::ranges::to<Array>());
    }

    Value_Ptr operator()(this const auto self, const boost::json::object& obj)
    {
        Map result;
        for (const auto& [k, v] : obj)
        {
            result.try_emplace(Value::create(String{k}),
                               boost::json::visit(self, v));
        }
        return Value::create(Value::trusted, std::move(result));
    }

} constexpr static decode_json_impl;

BUILTIN(decode)
{
    REQUIRE_ARGS("json.decode", TYPES(String));
    boost::system::error_code ec;
    boost::json::parse_options opts{
        .max_depth = 1024,
        .allow_comments = true,
        .allow_trailing_commas = true,
    };
    auto json = boost::json::parse(GET(0, String), ec, {}, opts);
    if (ec)
        throw Frost_Recoverable_Error{ec.message()};

    return boost::json::visit(decode_json_impl, json);
}

struct Encode_Json_Impl
{

    boost::json::value operator()(this const auto, const Null&)
    {
        return {};
    }

    boost::json::value operator()(this const auto, const String& str)
    {
        return {str};
    }

    boost::json::value operator()(this const auto,
                                  const Frost_Primitive auto& value)
    {
        return value;
    }

    boost::json::value operator()(this const auto self, const Array& arr)
    {
        return arr
               | std::views::transform([&](const Value_Ptr& elem) {
                     return elem->visit(self);
                 })
               | std::ranges::to<boost::json::array>();
    }

    boost::json::value operator()(this const auto self, const Map& map)
    {
        boost::json::object result;

        for (const auto& [k, v] : map)
        {
            if (not k->is<String>())
            {
                throw Frost_Recoverable_Error{
                    fmt::format("Map with non-String key: \"{}\" cannot be "
                                "serialized to JSON",
                                k->to_internal_string())};
            }

            result.emplace(k->raw_get<String>(), v->visit(self));
        }

        return result;
    }

    boost::json::value operator()(this const auto, const Function&)
    {
        throw Frost_Recoverable_Error{"Cannot serialize Function to JSON"};
    }

} constexpr static encode_json_impl;

BUILTIN(encode)
{
    return Value::create(serialize(args.at(0)->visit(encode_json_impl)));
}
} // namespace json

STDLIB_MODULE(json, ENTRY(decode, 1), ENTRY(encode, 1))

} // namespace frst
