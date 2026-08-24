#ifndef FROST_AST_MATCH_PATTERN_HPP
#define FROST_AST_MATCH_PATTERN_HPP

#include <frost/ast/ast-node.hpp>
#include <frost/execution-context.hpp>

namespace frst::ast
{

//! @brief Base class for the match-pattern subtree.
//!
//! A pattern attempts to match a value, binding any names declared by the
//! pattern directly into the given execution context as it goes. On a
//! failed match, partial bindings may remain in the context -- it is the
//! caller's responsibility (i.e. the Match node) to provide a per-arm
//! scratch scope that is discarded if the match fails and retained if it
//! succeeds.
class Match_Pattern : public AST_Node
{
  public:
    using Ptr = std::unique_ptr<Match_Pattern>;

    Match_Pattern(const Source_Range& source_range)
        : AST_Node(source_range)
    {
    }

    Match_Pattern() = delete;
    Match_Pattern(const Match_Pattern&) = delete;
    Match_Pattern(Match_Pattern&&) = delete;
    Match_Pattern& operator=(const Match_Pattern&) = delete;
    Match_Pattern& operator=(Match_Pattern&&) = delete;
    ~Match_Pattern() override = default;

    //! @brief Attempt to match this pattern against a value.
    //!
    //! Binds names into `ctx` as it proceeds. On failure, partial bindings
    //! may be present in `ctx`; the caller must scope `ctx` such that those
    //! bindings can be discarded.
    //!
    //! @returns true if the pattern matched, false if it did not.
    bool try_match(Execution_Context ctx, const Value_Ptr& value) const
    {
        auto guard = make_node_frame_guard(*this);
        return do_try_match(ctx, value);
    }

  protected:
    virtual bool do_try_match(Execution_Context ctx,
                              const Value_Ptr& value) const = 0;
};

} // namespace frst::ast

#endif
