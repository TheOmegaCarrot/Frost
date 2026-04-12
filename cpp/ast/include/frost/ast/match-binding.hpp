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

bool satisfies(std::optional<Type_Constraint> constraint,
               const Value_Ptr& value);

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

    std::generator<Symbol_Action> symbol_sequence() const final;

  protected:
    bool do_try_match(Execution_Context ctx,
                      const Value_Ptr& value) const final;
    std::string do_node_label() const final;

  private:
    std::optional<std::string> name_;
    std::optional<Type_Constraint> type_constraint_;
};

} // namespace frst::ast

#endif
