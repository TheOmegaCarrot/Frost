#ifndef FROST_COERCION_HPP
#define FROST_COERCION_HPP

#include <limits>
#include <optional>

#include "exceptions.hpp"
#include "types.hpp"

#include <fmt/format.h>

// ==========================================
// Type coercion tables
// ==========================================

namespace frst
{

template <Frost_Type T>
struct coerce_to;

#define COERCIONS_TO(Target)                                                   \
    template <>                                                                \
    struct coerce_to<Target>                                                   \
    {                                                                          \
        using target_t = Target;                                               \
        void completeness_check()                                              \
        {                                                                      \
            (*this)(Null{});                                                   \
            (*this)(Int{});                                                    \
            (*this)(Float{});                                                  \
            (*this)(Bool{});                                                   \
            (*this)(String{});                                                 \
            (*this)(Array{});                                                  \
            (*this)(Map{});                                                    \
            (*this)(Function{});                                               \
        }

#define COERCE(From)                                                           \
    static std::optional<target_t> operator()(                                 \
        [[maybe_unused]] const From& value)

#define NO_COERCE(From)                                                        \
    COERCE(From)                                                               \
    {                                                                          \
        return std::nullopt;                                                   \
    }

#define VALUE_COERCE(From)                                                     \
    COERCE(From)                                                               \
    {                                                                          \
        return value;                                                          \
    }

#define END_COERCIONS                                                          \
    }                                                                          \
    ;

COERCIONS_TO(Null)
VALUE_COERCE(Null)
NO_COERCE(Int)
NO_COERCE(Float)
NO_COERCE(Bool)
NO_COERCE(String)
NO_COERCE(Array)
NO_COERCE(Map)
NO_COERCE(Function)
END_COERCIONS

COERCIONS_TO(Int)
NO_COERCE(Null)
VALUE_COERCE(Int)
static std::optional<target_t> operator()([[maybe_unused]] const Float& value)
{
    if ((value < std::numeric_limits<Int>::lowest())
        || (value > static_cast<Float>(std::numeric_limits<Int>::max())))
        throw Frost_Recoverable_Error{
            fmt::format("Value {} is out of range of Int", value)};

    return value;
}
NO_COERCE(Bool)
NO_COERCE(String)
NO_COERCE(Array)
NO_COERCE(Map)
NO_COERCE(Function)
END_COERCIONS

COERCIONS_TO(Float)
NO_COERCE(Null)
VALUE_COERCE(Int)
VALUE_COERCE(Float)
NO_COERCE(Bool)
NO_COERCE(String)
NO_COERCE(Array)
NO_COERCE(Map)
NO_COERCE(Function)
END_COERCIONS

COERCIONS_TO(Bool)

COERCE(Null)
{
    return false;
}

COERCE(Int)
{
    return true;
}

COERCE(Float)
{
    return true;
}

VALUE_COERCE(Bool)

COERCE(String)
{
    return true;
}

COERCE(Array)
{
    return true;
}

COERCE(Map)
{
    return true;
}

COERCE(Function)
{
    return true;
}

END_COERCIONS

COERCIONS_TO(String)
NO_COERCE(Null)
NO_COERCE(Int)
NO_COERCE(Float)
NO_COERCE(Bool)
VALUE_COERCE(String)
NO_COERCE(Array)
NO_COERCE(Map)
NO_COERCE(Function)
END_COERCIONS

COERCIONS_TO(Array)
NO_COERCE(Null)
NO_COERCE(Int)
NO_COERCE(Float)
NO_COERCE(Bool)
NO_COERCE(String)
VALUE_COERCE(Array)
NO_COERCE(Map)
NO_COERCE(Function)
END_COERCIONS

COERCIONS_TO(Map)
NO_COERCE(Null)
NO_COERCE(Int)
NO_COERCE(Float)
NO_COERCE(Bool)
NO_COERCE(String)
NO_COERCE(Array)
VALUE_COERCE(Map)
NO_COERCE(Function)
END_COERCIONS

COERCIONS_TO(Function)
NO_COERCE(Null)
NO_COERCE(Int)
NO_COERCE(Float)
NO_COERCE(Bool)
NO_COERCE(String)
NO_COERCE(Array)
NO_COERCE(Map)
VALUE_COERCE(Function)
END_COERCIONS

#undef COERCIONS_TO
#undef COERCE
#undef VALUE_COERCE
#undef NO_COERCE
#undef END_COERCIONS

} // namespace frst

#endif
