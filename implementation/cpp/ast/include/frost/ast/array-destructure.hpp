#ifndef FROST_AST_ARRAY_DESTRUCTURE_HPP
#define FROST_AST_ARRAY_DESTRUCTURE_HPP

#include "expression.hpp"
#include "frost/value.hpp"
#include "statement.hpp"

#include <frost/symbol-table.hpp>

#include <flat_set>

namespace frst::ast
{

struct Discarded_Binding
{
    constexpr static std::string_view token = "_";
};

class Array_Destructure final : public Statement
{
  public:
    using Ptr = std::unique_ptr<Array_Destructure>;
    using Name = std::variant<Discarded_Binding, std::string>;

    Array_Destructure() = delete;
    Array_Destructure(const Array_Destructure&) = delete;
    Array_Destructure(Array_Destructure&&) = delete;
    Array_Destructure& operator=(const Array_Destructure&) = delete;
    Array_Destructure& operator=(Array_Destructure&&) = delete;
    ~Array_Destructure() final = default;

    Array_Destructure(std::vector<Name> names, std::optional<Name> rest_name,
                      Expression::Ptr expr, bool export_defs = false)
        : names_{std::move(names)}
        , rest_name_{std::move(rest_name)}
        , expr_{std::move(expr)}
        , export_defs_{export_defs}
    {
        std::flat_set<std::string_view> binding_names;

        auto duplicate_check = Overload{
            [](const Discarded_Binding&) {
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

    std::optional<Map> execute(Symbol_Table& table) const final
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
                fmt::format("Insufficient array elements to destructure: "
                            "required {} but got {}",
                            names_.size(), arr.size())};
        }

        if (not rest_name_ && arr.size() > names_.size())
        {
            throw Frost_Recoverable_Error{
                fmt::format("Too many array elements to destructure: required "
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
                                        exports.emplace(
                                            Value::create(auto{name}), val);
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

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        co_yield std::ranges::elements_of(expr_->symbol_sequence());

        for (const auto& binding : names_)
        {
            if (std::holds_alternative<std::string>(binding))
                co_yield Definition{std::get<std::string>(binding)};
        }

        if (rest_name_
            && std::holds_alternative<std::string>(rest_name_.value()))
            co_yield Definition{std::get<std::string>(rest_name_.value())};
    }

  protected:
    std::string node_label() const
    {
        return fmt::format(
            "{}Array_Destructure({})", export_defs_ ? "Export_" : "",
            names_
                | std::views::transform([](const auto& name) {
                      return name.visit(Overload{
                          [](const Discarded_Binding&) {
                              return Discarded_Binding::token;
                          },
                          [](const std::string& name) -> std::string_view {
                              return name;
                          }});
                  })
                | std::views::join_with(',')
                | std::ranges::to<std::string>());
    }

    std::generator<Child_Info> children() const
    {
        co_yield make_child(expr_);
    }

  private:
    std::vector<Name> names_;
    std::optional<Name> rest_name_;
    Expression::Ptr expr_;
    bool export_defs_;
};

} // namespace frst::ast

#endif
