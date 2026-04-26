#include <frost/builtin.hpp>
#include <frost/builtins-common.hpp>
#include <frost/value.hpp>

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
            fmt::format("json.decode: Value {} is out of range", u)};
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
        throw Frost_Recoverable_Error{
            fmt::format("json.decode: {}", ec.message())};

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
                throw Frost_Recoverable_Error{fmt::format(
                    "json.encode: Map with non-String key: \"{}\" cannot be "
                    "serialized to JSON",
                    k->to_internal_string())};
            }

            result.emplace(k->raw_get<String>(), v->visit(self));
        }

        return result;
    }

    boost::json::value operator()(this const auto, const Function&)
    {
        throw Frost_Recoverable_Error{
            "json.encode: Cannot serialize Function to JSON"};
    }

} constexpr static encode_json_impl;

BUILTIN(encode)
{
    REQUIRE_ARGS("json.encode", ANY);
    return Value::create(serialize(args.at(0)->visit(encode_json_impl)));
}

void pretty_format(std::string& out, const boost::json::value& val,
                   std::size_t indent, std::size_t depth)
{
    if (val.is_null()
        || val.is_bool()
        || val.is_int64()
        || val.is_uint64()
        || val.is_double()
        || val.is_string())
    {
        out += serialize(val);
        return;
    }

    std::string inner_pad(indent * (depth + 1), ' ');
    std::string outer_pad(indent * depth, ' ');

    if (val.is_array())
    {
        const auto& arr = val.get_array();
        if (arr.empty())
        {
            out += "[]";
            return;
        }

        out += "[\n";
        for (std::size_t i = 0; i < arr.size(); ++i)
        {
            out += inner_pad;
            pretty_format(out, arr[i], indent, depth + 1);
            if (i + 1 < arr.size())
                out += ',';
            out += '\n';
        }
        out += outer_pad;
        out += ']';
        return;
    }

    const auto& obj = val.get_object();
    if (obj.empty())
    {
        out += "{}";
        return;
    }

    out += "{\n";
    std::size_t i = 0;
    for (const auto& [k, v] : obj)
    {
        out += inner_pad;
        out += serialize(boost::json::value{k});
        out += ": ";
        pretty_format(out, v, indent, depth + 1);
        if (++i < obj.size())
            out += ',';
        out += '\n';
    }
    out += outer_pad;
    out += '}';
}

BUILTIN(encode_pretty)
{
    REQUIRE_ARGS("json.encode_pretty", ANY, PARAM("indent", TYPES(Int)));

    auto indent = args.at(1)->raw_get<Int>();
    if (indent < 0)
        throw Frost_Recoverable_Error{
            "json.encode_pretty: requires a non-negative indent"};

    auto json = args.at(0)->visit(encode_json_impl);

    std::string result;
    pretty_format(result, json, static_cast<std::size_t>(indent), 0);
    return Value::create(std::move(result));
}

} // namespace json

STDLIB_MODULE(json, ENTRY(decode), ENTRY(encode), ENTRY(encode_pretty))

} // namespace frst
