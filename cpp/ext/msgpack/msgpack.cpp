#include <frost/data-builtin.hpp>
#include <frost/extensions-common.hpp>

#include <msgpack.hpp>

#include <cmath>
#include <limits>

namespace frst
{
namespace msgpack
{

namespace
{

struct MsgPack_Binary
{
    Value_Ptr data;
};

struct MsgPack_Special_Float
{
    double d;
};

Function make_binary(Value_Ptr data)
{
    return std::make_shared<Data_Builtin<MsgPack_Binary>>(
        [data](builtin_args_t args) {
            REQUIRE_NULLARY("msgpack.binary");
            return data;
        },
        "msgpack.binary", MsgPack_Binary{data});
}

Function make_special_float(double d)
{
    auto repr = Value::create([&] -> std::string {
        if (std::isnan(d))
            return "nan";
        else if (d > 0)
            return "inf";
        else
            return "-inf";
    }());

    return std::make_shared<Data_Builtin<MsgPack_Special_Float>>(
        [repr = std::move(repr)](builtin_args_t args) {
            REQUIRE_NULLARY("msgpack.special_float");
            return repr;
        },
        "msgpack.special_float", MsgPack_Special_Float{d});
}

Value_Ptr decode_object(const ::msgpack::object& obj)
{
    switch (obj.type)
    {
    case ::msgpack::type::NIL:
        return Value::null();

    case ::msgpack::type::BOOLEAN:
        return Value::create(Bool{obj.via.boolean});

    case ::msgpack::type::POSITIVE_INTEGER: {
        auto v = obj.via.u64;
        if (v > static_cast<uint64_t>(std::numeric_limits<Int>::max()))
            throw Frost_Recoverable_Error{fmt::format(
                "msgpack.decode: unsigned integer {} exceeds Int range", v)};
        return Value::create(Int{static_cast<Int>(v)});
    }

    case ::msgpack::type::NEGATIVE_INTEGER:
        return Value::create(Int{obj.via.i64});

    case ::msgpack::type::FLOAT32:
    case ::msgpack::type::FLOAT64: {
        double d = obj.via.f64;
        if (std::isnan(d) || std::isinf(d))
            return Value::create(make_special_float(d));
        return Value::create(d);
    }

    case ::msgpack::type::STR:
        return Value::create(String{obj.via.str.ptr, obj.via.str.size});

    case ::msgpack::type::BIN:
        return Value::create(make_binary(
            Value::create(String{obj.via.bin.ptr, obj.via.bin.size})));

    case ::msgpack::type::ARRAY: {
        Array arr;
        arr.reserve(obj.via.array.size);
        for (uint32_t i = 0; i < obj.via.array.size; ++i)
            arr.push_back(decode_object(obj.via.array.ptr[i]));
        return Value::create(std::move(arr));
    }

    case ::msgpack::type::MAP: {
        Map result;
        for (uint32_t i = 0; i < obj.via.map.size; ++i)
        {
            auto& kv = obj.via.map.ptr[i];
            auto key = decode_object(kv.key);
            if (not key->is_primitive() || key->is<Null>())
                throw Frost_Recoverable_Error{
                    fmt::format("msgpack.decode: unsupported map key type {}. "
                                "Frost map keys must be non-null primitives",
                                key->type_name())};
            result.try_emplace(std::move(key), decode_object(kv.val));
        }
        return Value::create(Value::trusted, std::move(result));
    }

    case ::msgpack::type::EXT:
        throw Frost_Recoverable_Error{
            fmt::format("msgpack.decode: extension type {} is not supported",
                        obj.via.ext.type())};
    }

    THROW_UNREACHABLE;
}

void encode_into(::msgpack::packer<::msgpack::sbuffer>& pk,
                 const Value_Ptr& val);

struct Encode_MsgPack
{
    ::msgpack::packer<::msgpack::sbuffer>& pk;

    void operator()(const Null&) const
    {
        pk.pack_nil();
    }

    void operator()(const Bool& b) const
    {
        b ? pk.pack_true() : pk.pack_false();
    }

    void operator()(const Int& i) const
    {
        pk.pack_int64(i);
    }

    void operator()(const Float& f) const
    {
        pk.pack_double(f);
    }

    void operator()(const String& s) const
    {
        pk.pack_str(static_cast<uint32_t>(s.size()));
        pk.pack_str_body(s.data(), static_cast<uint32_t>(s.size()));
    }

    void operator()(const Array& arr) const
    {
        pk.pack_array(static_cast<uint32_t>(arr.size()));
        for (const auto& elem : arr)
            encode_into(pk, elem);
    }

    void operator()(const Map& map) const
    {
        pk.pack_map(static_cast<uint32_t>(map.size()));
        for (const auto& [k, v] : map)
        {
            encode_into(pk, k);
            encode_into(pk, v);
        }
    }

    void operator()(const Function& fn) const
    {
        if (auto* db =
                dynamic_cast<const Data_Builtin<MsgPack_Binary>*>(fn.get()))
        {
            const auto& bytes = db->data().data->raw_get<String>();
            pk.pack_bin(static_cast<uint32_t>(bytes.size()));
            pk.pack_bin_body(bytes.data(), static_cast<uint32_t>(bytes.size()));
        }
        else if (auto* db =
                     dynamic_cast<const Data_Builtin<MsgPack_Special_Float>*>(
                         fn.get()))
        {
            pk.pack_double(db->data().d);
        }
        else
        {
            throw Frost_Recoverable_Error{
                "msgpack.encode: cannot serialize Function to MessagePack"};
        }
    }
};

void encode_into(::msgpack::packer<::msgpack::sbuffer>& pk,
                 const Value_Ptr& val)
{
    val->visit(Encode_MsgPack{pk});
}

} // namespace

BUILTIN(decode)
{
    REQUIRE_ARGS("msgpack.decode", PARAM("data", TYPES(String)));

    const auto& data = GET(0, String);

    try
    {
        auto handle = ::msgpack::unpack(data.data(), data.size());
        return decode_object(handle.get());
    }
    catch (const ::msgpack::unpack_error& e)
    {
        throw Frost_Recoverable_Error{
            fmt::format("msgpack.decode: {}", e.what())};
    }
}

BUILTIN(encode)
{
    REQUIRE_ARGS("msgpack.encode", ANY);

    ::msgpack::sbuffer buf;
    ::msgpack::packer pk{buf};

    encode_into(pk, args.at(0));

    return Value::create(String{buf.data(), buf.size()});
}

BUILTIN(binary)
{
    REQUIRE_ARGS("msgpack.binary", PARAM("data", TYPES(String)));
    return Value::create(make_binary(args.at(0)));
}

BUILTIN(special_float)
{
    REQUIRE_ARGS("msgpack.special_float", TYPES(String));

    using lim = std::numeric_limits<double>;

    const auto& str = GET(0, String);
    if (str == "nan")
        return Value::create(make_special_float(lim::quiet_NaN()));
    else if (str == "inf")
        return Value::create(make_special_float(lim::infinity()));
    else if (str == "-inf")
        return Value::create(make_special_float(-lim::infinity()));

    throw Frost_Recoverable_Error{fmt::format(
        "msgpack.special_float: Input must be 'nan', 'inf', or '-inf', got: {}",
        str)};
}

} // namespace msgpack

REGISTER_EXTENSION(msgpack, ENTRY(decode), ENTRY(encode), ENTRY(binary),
                   ENTRY(special_float))

} // namespace frst
