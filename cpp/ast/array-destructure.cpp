#include <frost/ast/array-destructure.hpp>

#include <fmt/ranges.h>

#include <flat_set>

#include <ranges>

using namespace frst;
using namespace frst::ast;

Array_Destructure::Array_Destructure(Source_Range source_range,
                                     std::vector<Name> names,
                                     std::optional<Name> rest_name,
                                     Expression::Ptr expr, bool export_defs)
    : Statement(source_range)
    , names_{std::move(names)}
    , rest_name_{std::move(rest_name)}
    , expr_{std::move(expr)}
    , export_defs_{export_defs}
{
    std::flat_set<std::string_view> binding_names;

    auto duplicate_check =
        Overload{[](const Discarded_Binding&) {
                 },
                 [&](const std::string& name) {
                     if (binding_names.contains(name))
                     {
                         throw Frost_Unrecoverable_Error{fmt::format(
                             "Duplicate destructuring binding name: {}", name)};
                     }
                     binding_names.insert(name);
                 }};

    for (const auto& name : names_)
    {
        name.visit(duplicate_check);
    }

    if (rest_name_)
        rest_name_->visit(duplicate_check);
}

void Array_Destructure::do_execute(Execution_Context& ctx) const
{
    Value_Ptr expr_result = expr_->evaluate(ctx.as_eval());

    if (not expr_result->is<Array>())
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Cannot destructure {} to Array", expr_result->type_name())};
    }

    const auto& arr = expr_result->raw_get<Array>();

    if (arr.size() < names_.size())
    {
        throw Frost_Recoverable_Error{
            fmt::format("Insufficient Array elements to destructure: "
                        "required {} but got {}",
                        names_.size(), arr.size())};
    }

    if (not rest_name_ && arr.size() > names_.size())
    {
        throw Frost_Recoverable_Error{
            fmt::format("Too many Array elements to destructure: required "
                        "{} but got {}",
                        names_.size(), arr.size())};
    }

    for (const auto& [name, val] : std::views::zip(names_, arr))
    {
        name.visit(Overload{
            [](const Discarded_Binding&) {
            },
            [&](const std::string& name) {
                ctx.symbols.define(name, val);
            },
        });
    }

    if (rest_name_)
    {
        rest_name_->visit(Overload{
            [](const Discarded_Binding&) {
            },
            [&](const std::string& name) {
                auto val = Value::create(arr
                                         | std::views::drop(names_.size())
                                         | std::ranges::to<Array>());

                ctx.symbols.define(name, val);
            },
        });
    }
}

std::generator<AST_Node::Symbol_Action> Array_Destructure::symbol_sequence()
    const
{
    co_yield std::ranges::elements_of(expr_->symbol_sequence());

    for (const auto& binding : names_)
    {
        if (std::holds_alternative<std::string>(binding))
            co_yield Definition{.name = std::get<std::string>(binding),
                                .exported = export_defs_};
    }

    if (rest_name_ && std::holds_alternative<std::string>(rest_name_.value()))
        co_yield Definition{.name = std::get<std::string>(rest_name_.value()),
                            .exported = export_defs_};
}

std::string Array_Destructure::do_node_label() const
{
    auto name_of = [](const Name& name) -> std::string_view {
        return name.visit(
            Overload{[](const Discarded_Binding&) -> std::string_view {
                         return Discarded_Binding::token;
                     },
                     [](const std::string& n) -> std::string_view {
                         return n;
                     }});
    };

    return fmt::format(
        "{}Array_Destructure({}{}{}{})", export_defs_ ? "Export_" : "",
        fmt::join(names_ | std::views::transform(name_of), ","),
        rest_name_ && !names_.empty() ? "," : "", rest_name_ ? "..." : "",
        rest_name_ ? name_of(*rest_name_) : std::string_view{});
}

std::generator<AST_Node::Child_Info> Array_Destructure::children() const
{
    co_yield make_child(expr_);
}
