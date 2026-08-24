#include <frost/ast/match-alternative.hpp>

#include <flat_set>

#include <fmt/format.h>
#include <fmt/ranges.h>

using namespace frst::ast;

namespace
{

std::flat_set<std::string> bindings_of(const Match_Pattern::Ptr& node)
{
    std::flat_set<std::string> result;
    for (const AST_Node::Symbol_Action& action : node->symbol_sequence())
    {
        action.visit(frst::Overload{
            [&](const AST_Node::Definition& def) {
                result.insert(def.name);
            },
            [](const AST_Node::Usage&) {

            },
        });
    }
    return result;
}

void merge_tables(frst::Symbol_Table& destination,
                  const frst::Symbol_Table& source)
{
    for (const auto& name : source.names())
    {
        destination.define(std::string{name}, source.lookup(std::string{name}));
    }
}

} // namespace

Match_Alternative::Match_Alternative(
    const Source_Range& source_range,
    std::vector<Match_Pattern::Ptr> alternatives)
    : Match_Pattern(source_range)
    , alternatives_{std::move(alternatives)}
{
    // If there aren't at least 2 alternatives,
    // there's a bug in the parser
    if (alternatives_.size() < 2)
        THROW_UNREACHABLE;

    auto definitive_bindings = bindings_of(alternatives_.front());

    for (const auto& [i, alternative] :
         alternatives_ | std::views::enumerate | std::views::drop(1))
    {
        auto alt_bindings = bindings_of(alternative);
        if (alt_bindings != definitive_bindings)
        {
            throw Frost_Unrecoverable_Error{fmt::format(
                "Match alternatives must bind the same names, "
                "but alternative 1 binds {{{}}} while alternative {} "
                "binds {{{}}}",
                fmt::join(definitive_bindings, ", "), i + 1,
                fmt::join(alt_bindings, ", "))};
        }
    }
}

std::generator<AST_Node::Symbol_Action> Match_Alternative::symbol_sequence()
    const
{
    for (const auto& node : alternatives_)
        co_yield std::ranges::elements_of(node->symbol_sequence());
}

std::generator<AST_Node::Child_Info> Match_Alternative::children() const
{
    for (const auto& alternative : alternatives_)
        co_yield make_child(alternative);
}

bool Match_Alternative::do_try_match(Execution_Context ctx,
                                     const Value_Ptr& value) const
{
    for (const auto& alternative : alternatives_)
    {
        Symbol_Table scratch_table{&ctx.symbols};
        Execution_Context scratch_ctx{.symbols = scratch_table};

        if (alternative->try_match(scratch_ctx, value))
        {
            merge_tables(ctx.symbols, scratch_table);
            return true;
        }
    }
    return false;
}
