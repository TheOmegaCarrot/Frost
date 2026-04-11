#ifndef FROST_AST_MATCH_MAP_HPP
#define FROST_AST_MATCH_MAP_HPP

#include <frost/ast/expression.hpp>
#include <frost/ast/match-pattern.hpp>

#include <utility>
#include <vector>

namespace frst::ast
{

//! @brief Match a Map value by key/pattern pairs.
//!
//! The match target must be a Map. Each declared entry evaluates its key
//! expression to produce a lookup key and tries its sub-pattern against
//! the value at that key. A missing key yields `null` for the sub-pattern,
//! matching Frost's uniform "missing map key = null" semantics used by
//! destructuring and indexing. The overall match succeeds only when every
//! declared entry's sub-pattern succeeds in order.
//!
//! Extra keys in the match target that aren't declared in the pattern are
//! permitted; Match_Map only checks the keys it names.
//!
//! Users who want to require a key to be non-null can express it via a
//! type constraint on the binding (e.g. `{age: a is Int}`), and users who
//! need to literally distinguish "key absent" from "key present, value
//! null" can fall back to a guard using `has(m, key)`.
//!
//! A key expression that evaluates to an invalid Map key type is a
//! malformed pattern (but a recoverable error), not a match failure.
//! This matches Destructure_Map's policy.
class Match_Map final : public Match_Pattern
{
  public:
    using Ptr = std::unique_ptr<Match_Map>;

    struct Element
    {
        Expression::Ptr key;
        Match_Pattern::Ptr pattern;
    };

    Match_Map() = delete;
    Match_Map(const Match_Map&) = delete;
    Match_Map(Match_Map&&) = delete;
    Match_Map& operator=(const Match_Map&) = delete;
    Match_Map& operator=(Match_Map&&) = delete;
    ~Match_Map() final = default;

    Match_Map(const Source_Range& source_range, std::vector<Element> elements)
        : Match_Pattern{source_range}
        , elements_{std::move(elements)}
    {
    }

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        for (const auto& [key_expr, pattern] : elements_)
        {
            co_yield std::ranges::elements_of(key_expr->symbol_sequence());
            co_yield std::ranges::elements_of(pattern->symbol_sequence());
        }
    }

    std::generator<Child_Info> children() const final
    {
        for (const auto& [i, elem] : std::views::enumerate(elements_))
        {
            const auto& [key_expr, pattern] = elem;
            co_yield make_child(key_expr, fmt::format("Key {}", i + 1));
            co_yield make_child(pattern, fmt::format("Pattern {}", i + 1));
        }
    }

  protected:
    bool do_try_match(Execution_Context ctx, const Value_Ptr& value) const final
    {
        if (not value->is<frst::Map>())
            return false;

        const frst::Map& match_target = value->raw_get<frst::Map>();

        for (const auto& [key_expr, pattern] : elements_)
        {
            auto key = key_expr->evaluate(ctx.as_eval());
            if (key->visit([]<typename T>(const T&) {
                    return not Frost_Map_Key<T>;
                }))
            {
                throw Frost_Recoverable_Error{fmt::format(
                    "Map match key expressions must be valid Map keys, "
                    "got: {}",
                    key->type_name())};
            }

            auto itr = match_target.find(key);
            auto sub_value =
                (itr == match_target.end()) ? Value::null() : itr->second;

            if (not pattern->try_match(ctx, sub_value))
                return false;
        }

        return true;
    }

    std::string do_node_label() const final
    {
        return "Match_Map";
    }

  private:
    std::vector<Element> elements_;
};

} // namespace frst::ast

#endif
