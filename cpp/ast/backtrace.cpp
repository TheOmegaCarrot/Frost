#include <frost/ast/statement.hpp>
#include <frost/backtrace.hpp>
#include <frost/value.hpp>

#include <fmt/format.h>

#include <ranges>

namespace frst
{

void Backtrace_State::snapshot_if_needed()
{
    if (!snapshot_.empty() || frames_.empty())
        return;

    snapshot_.reserve(frames_.size());

    for (const auto& frame : frames_ | std::views::reverse)
    {
        std::visit(
            Overload{
                [&](const AST_Frame& f) {
                    snapshot_.emplace_back(fmt::format(
                        "{} [{}]", f.node->node_label(), f.node->source_range()));
                },
                [&](const Call_Frame& f) {
                    snapshot_.emplace_back(
                        fmt::format("Call ({})", f.function_name));
                },
                [&](const Import_Frame& f) {
                    snapshot_.emplace_back(
                        fmt::format("Import Boundary ({})", f.module_spec));
                },
                [&](const Iterative_Frame& f) {
                    snapshot_.emplace_back(
                        fmt::format("{} ({})", f.operation, f.function_name));
                },
            },
            frame);
    }
}

} // namespace frst
