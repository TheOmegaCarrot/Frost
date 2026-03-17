#include <frost/ast/statement.hpp>
#include <frost/backtrace.hpp>
#include <frost/value.hpp>

#include <fmt/format.h>

#include <ranges>

namespace frst
{

void Backtrace_State::do_snapshot()
{
    snapshot_.reserve(frames_.size());

    for (const auto& frame : frames_ | std::views::reverse)
    {
        snapshot_.emplace_back(frame.visit(Overload{
            [](const AST_Frame& f) {
                return fmt::format("{} [{}]", f.node->node_label(),
                                   f.node->source_range());
            },
            [](const Call_Frame& f) {
                return fmt::format("Call ({})", f.function_name);
            },
            [](const Import_Frame& f) {
                return fmt::format("Import Boundary ({})", f.module_spec);
            },
            [](const Iterative_Frame& f) {
                return fmt::format("{} ({})", f.operation, f.function_name);
            },
        }));
    }
}

} // namespace frst
