#ifndef FROST_AST_MATCH_ARRAY_HPP
#define FROST_AST_MATCH_ARRAY_HPP

#include <frost/ast/match-pattern.hpp>

#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace frst::ast
{

//! @brief Match an Array value element-wise, with an optional rest clause.
//!
//! Without a rest clause: the scrutinee must be an Array with exactly as
//! many elements as there are element patterns. Each pattern is tried
//! against the corresponding element in order.
//!
//! With a rest clause: the scrutinee must be an Array with *at least* as
//! many elements as there are element patterns. The leading elements are
//! matched positionally against the element patterns; the remaining
//! elements are gathered into a fresh Array. If the rest clause names an
//! identifier, that Array is bound to the name; if it is a discard, the
//! remainder is dropped.
class Match_Array final : public Match_Pattern
{
  public:
    using Ptr = std::unique_ptr<Match_Array>;

    //! @brief Describes an `...rest` or `..._` clause on an array pattern.
    //!
    //! `name == nullopt` means the rest is discarded (`..._`);
    //! `name.has_value()` means the remainder is bound to that identifier.
    struct Rest
    {
        std::optional<std::string> name;
    };

    Match_Array() = delete;
    Match_Array(const Match_Array&) = delete;
    Match_Array(Match_Array&&) = delete;
    Match_Array& operator=(const Match_Array&) = delete;
    Match_Array& operator=(Match_Array&&) = delete;
    ~Match_Array() final = default;

    Match_Array(const Source_Range& source_range,
                std::vector<Match_Pattern::Ptr> subpatterns,
                std::optional<Rest> rest)
        : Match_Pattern{source_range}
        , subpatterns_{std::move(subpatterns)}
        , rest_{std::move(rest)}
    {
    }

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        for (const auto& pattern : subpatterns_)
            co_yield std::ranges::elements_of(pattern->symbol_sequence());

        if (rest_ && rest_->name)
            co_yield Definition{.name = rest_->name.value(), .exported = false};
    }

    std::generator<Child_Info> children() const final
    {
        for (const auto& pattern : subpatterns_)
            co_yield make_child(pattern);
    }

  protected:
    bool do_try_match(Execution_Context ctx, const Value_Ptr& value) const final
    {
        if (not value->is<Array>())
            return false;

        const auto& arr = value->raw_get<Array>();

        // Without a rest clause the sizes must match exactly; with one the
        // scrutinee may be longer than the element patterns.
        if (rest_)
        {
            if (arr.size() < subpatterns_.size())
                return false;
        }
        else
        {
            if (arr.size() != subpatterns_.size())
                return false;
        }

        for (const auto& [pat, elem] : std::views::zip(subpatterns_, arr))
        {
            if (not pat->try_match(ctx, elem))
                return false;
        }

        if (rest_ && rest_->name)
        {
            auto tail = arr
                        | std::views::drop(subpatterns_.size())
                        | std::ranges::to<Array>();

            ctx.symbols.define(rest_->name.value(),
                               Value::create(std::move(tail)));
        }

        return true;
    }

    std::string do_node_label() const final
    {
        if (rest_)
            return fmt::format("Match_Array(...{})", rest_->name.value_or("_"));
        return "Match_Array";
    }

  private:
    std::vector<Match_Pattern::Ptr> subpatterns_;
    std::optional<Rest> rest_;
};

} // namespace frst::ast

#endif
