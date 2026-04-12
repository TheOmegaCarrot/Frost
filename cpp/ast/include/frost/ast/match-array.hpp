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

    std::generator<Symbol_Action> symbol_sequence() const final;
    std::generator<Child_Info> children() const final;

  protected:
    bool do_try_match(Execution_Context ctx,
                      const Value_Ptr& value) const final;
    std::string do_node_label() const final;

  private:
    std::vector<Match_Pattern::Ptr> subpatterns_;
    std::optional<Rest> rest_;
};

} // namespace frst::ast

#endif
