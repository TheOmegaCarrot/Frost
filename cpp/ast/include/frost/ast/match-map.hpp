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
//! expression to produce a lookup key. If the key is absent from the
//! match target the match fails immediately; the sub-pattern is never
//! consulted. If the key is present, its sub-pattern is tried against
//! the corresponding value. The overall match succeeds only when every
//! declared entry's key is found and its sub-pattern succeeds.
//!
//! Extra keys in the match target that aren't declared in the pattern are
//! permitted; Match_Map only checks the keys it names.
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

    Match_Map(const Source_Range& source_range, std::vector<Element> elements,
              std::optional<std::string> bind_whole_name)
        : Match_Pattern{source_range}
        , elements_{std::move(elements)}
        , bind_whole_name_{std::move(bind_whole_name)}
    {
    }

    std::generator<Symbol_Action> symbol_sequence() const final;
    std::generator<Child_Info> children() const final;

  protected:
    bool do_try_match(Execution_Context ctx,
                      const Value_Ptr& value) const final;
    std::string do_node_label() const final;

  private:
    std::vector<Element> elements_;
    std::optional<std::string> bind_whole_name_;
};

} // namespace frst::ast

#endif
