#include <frost/ast/statement.hpp>
#include <frost/backtrace.hpp>
#include <frost/value.hpp>

#include <fmt/format.h>

namespace frst
{

void Backtrace_State::snapshot_if_needed()
{
    if (!snapshot_.empty() || frames_.empty())
        return;

    snapshot_.reserve(frames_.size());

    for (const auto& frame : frames_)
    {
        std::visit(
            Overload{
                [&](const AST_Frame& f) {
                    snapshot_.emplace_back(
                        Resolved_AST_Frame{.node_label = f.node->node_label(),
                                           .source_range = fmt::format(
                                               "{}", f.node->source_range())});
                },
                [&](const Call_Frame& f) { snapshot_.emplace_back(f); },
                [&](const Import_Frame& f) { snapshot_.emplace_back(f); },
                [&](const Iterative_Frame& f) { snapshot_.emplace_back(f); },
            },
            frame);
    }
}

} // namespace frst
