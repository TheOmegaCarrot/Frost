#include <frost/ast/array-destructure.hpp>

#include <flat_set>

#include <ranges>

using namespace frst;
using namespace frst::ast;

Array_Destructure::Array_Destructure(std::vector<Name> names,
                                     std::optional<Name> rest_name,
                                     Expression::Ptr expr, bool export_defs)
    : names_{std::move(names)}
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

std::optional<Map> Array_Destructure::execute(Symbol_Table& table) const
{
    Value_Ptr expr_result = expr_->evaluate(table);

    Map exports;

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
        name.visit(Overload{[](const Discarded_Binding&) {
                            },
                            [&](const std::string& name) {
                                table.define(name, val);
                                if (export_defs_)
                                {
                                    exports.emplace(Value::create(auto{name}),
                                                    val);
                                }
                            }});
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

                table.define(name, val);

                if (export_defs_)
                    exports.emplace(Value::create(auto{name}), val);
            }});
    }

    if (export_defs_)
        return exports;
    else
        return std::nullopt;
}

std::generator<Statement::Symbol_Action> Array_Destructure::symbol_sequence()
    const
{
    co_yield std::ranges::elements_of(expr_->symbol_sequence());

    for (const auto& binding : names_)
    {
        if (std::holds_alternative<std::string>(binding))
            co_yield Definition{std::get<std::string>(binding)};
    }

    if (rest_name_ && std::holds_alternative<std::string>(rest_name_.value()))
        co_yield Definition{std::get<std::string>(rest_name_.value())};
}

std::string Array_Destructure::node_label() const
{
    return fmt::format(
        "{}Array_Destructure({})", export_defs_ ? "Export_" : "",
        names_
            | std::views::transform([](const auto& name) {
                  return name.visit(
                      Overload{[](const Discarded_Binding&) {
                                   return Discarded_Binding::token;
                               },
                               [](const std::string& name) -> std::string_view {
                                   return name;
                               }});
              })
            | std::views::join_with(',')
            | std::ranges::to<std::string>());
}

std::generator<Statement::Child_Info> Array_Destructure::children() const
{
    co_yield make_child(expr_);
}
