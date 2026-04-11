#ifndef FROST_AST_MATCH_BINDING_HPP
#define FROST_AST_MATCH_BINDING_HPP

#include <frost/ast/match-pattern.hpp>

#include <optional>
#include <string_view>
#include <utility>

namespace frst::ast
{

// The string form of each value matches its in-code name exactly:
//     to_string(Type_Constraint::Float) == "Float"
//     from_string("Float")               == Type_Constraint::Float
//
// `Unconstrained` is the sentinel for a binding with no `is TYPE` clause; it
// has no user-facing surface syntax.
#define X_TYPE_CONSTRAINTS                                                     \
    X(Null)                                                                    \
    X(Int)                                                                     \
    X(Float)                                                                   \
    X(Bool)                                                                    \
    X(String)                                                                  \
    X(Array)                                                                   \
    X(Map)                                                                     \
    X(Function)                                                                \
    X(Primitive)                                                               \
    X(Numeric)                                                                 \
    X(Structured)                                                              \
    X(Nonnull)

enum class Type_Constraint
{
#define X(NAME) NAME,
    // clang-format off
    X_TYPE_CONSTRAINTS
#undef X // clang-format keeps trying to put a comma here
    // clang-format on
};
} // namespace frst::ast

namespace frst::ast::TC
{
constexpr std::string_view to_string(Type_Constraint c)
{
    switch (c)
    {
#define X(NAME)                                                                \
    case Type_Constraint::NAME:                                                \
        return #NAME;
        X_TYPE_CONSTRAINTS
#undef X
    }
    THROW_UNREACHABLE;
}

constexpr std::optional<Type_Constraint> from_string(std::string_view s)
{
#define X(NAME)                                                                \
    if (s == #NAME)                                                            \
        return Type_Constraint::NAME;
    X_TYPE_CONSTRAINTS
#undef X
    return std::nullopt;
}

inline bool satisfies(std::optional<Type_Constraint> constraint,
                      const Value_Ptr& value)
{
    if (not constraint)
        return true;

    return value->visit([&]<typename Type>(const Type&) {
        switch (constraint.value())
        {
        case Type_Constraint::Null:
            return std::same_as<Null, Type>;
        case Type_Constraint::Int:
            return std::same_as<Int, Type>;
        case Type_Constraint::Float:
            return std::same_as<Float, Type>;
        case Type_Constraint::Bool:
            return std::same_as<Bool, Type>;
        case Type_Constraint::String:
            return std::same_as<String, Type>;
        case Type_Constraint::Array:
            return std::same_as<Array, Type>;
        case Type_Constraint::Map:
            return std::same_as<Map, Type>;
        case Type_Constraint::Function:
            return std::same_as<Function, Type>;
        case Type_Constraint::Primitive:
            return Frost_Primitive<Type>;
        case Type_Constraint::Numeric:
            return Frost_Numeric<Type>;
        case Type_Constraint::Structured:
            return Frost_Structured<Type>;
        case Type_Constraint::Nonnull:
            return not std::same_as<Null, Type>;
        }
        THROW_UNREACHABLE;
    });
}

} // namespace frst::ast::TC

template <>
struct fmt::formatter<frst::ast::Type_Constraint>
    : fmt::formatter<std::string_view>
{
    auto format(frst::ast::Type_Constraint c, fmt::format_context& ctx) const
    {
        return fmt::format_to(ctx.out(), "{}", frst::ast::TC::to_string(c));
    }
};

namespace frst::ast
{
class Match_Binding final : public Match_Pattern
{

  public:
    using Ptr = std::unique_ptr<Match_Binding>;

    Match_Binding(const Source_Range& source_range,
                  std::optional<std::string> name,
                  std::optional<Type_Constraint> type_constraint)
        : Match_Pattern(source_range)
        , name_{std::move(name)}
        , type_constraint_{type_constraint}
    {
    }

    Match_Binding() = delete;
    Match_Binding(const Match_Binding&) = delete;
    Match_Binding(Match_Binding&&) = delete;
    Match_Binding& operator=(const Match_Binding&) = delete;
    Match_Binding& operator=(Match_Binding&&) = delete;
    ~Match_Binding() final = default;

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        if (name_)
            co_yield Definition{.name = name_.value(), .exported = false};
    }

  protected:
    bool do_try_match(Execution_Context ctx, const Value_Ptr& value) const final
    {
        if (not TC::satisfies(type_constraint_, value))
            return false;

        if (name_)
            ctx.symbols.define(name_.value(), value);

        return true;
    }

    std::string do_node_label() const final
    {
        if (type_constraint_)
            return fmt::format("Match_Binding({} is {})", name_.value_or("_"),
                               type_constraint_.value());
        else
            return fmt::format("Match_Binding({})", name_.value_or("_"));
    }

  private:
    std::optional<std::string> name_;
    std::optional<Type_Constraint> type_constraint_;
};

} // namespace frst::ast

#endif
