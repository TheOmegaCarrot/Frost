#ifndef FROST_TYPE_STRINGS_HPP
#define FROST_TYPE_STRINGS_HPP

#include "types.hpp"

#include <boost/preprocessor/stringize.hpp>

// ==========================================
// Type name table
// ==========================================

namespace frst
{
template <Frost_Type T>
std::string_view type_str() = delete;

#define TYPE_STR_SPEC(T)                                                       \
    template <>                                                                \
    inline std::string_view type_str<T>()                                      \
    {                                                                          \
        return BOOST_PP_STRINGIZE(T);                                          \
    }

TYPE_STR_SPEC(Null)
TYPE_STR_SPEC(Int)
TYPE_STR_SPEC(Float)
TYPE_STR_SPEC(String)
TYPE_STR_SPEC(Bool)
TYPE_STR_SPEC(Array)
TYPE_STR_SPEC(Map)
TYPE_STR_SPEC(Function)

#undef TYPE_STR_SPEC

struct Type_Str_Fn
{
    template <Frost_Type T>
    static std::string_view operator()(const T&)
    {
        return type_str<T>();
    }
} constexpr inline type_str_fn;
} // namespace frst

#endif
