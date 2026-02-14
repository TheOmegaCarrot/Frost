#ifndef FROST_AST_MAP_DESTRUCTURE_HPP
#define FROST_AST_MAP_DESTRUCTURE_HPP

#include "expression.hpp"
#include "frost/value.hpp"
#include "statement.hpp"

#include <frost/symbol-table.hpp>

#include <flat_set>

namespace frst::ast
{

class Map_Destructure final : public Statement
{

  public:
    using Ptr = std::unique_ptr<Map_Destructure>;

    struct Element // { [key]: name }
    {
        Expression::Ptr key;
        std::string name;
    };

    Map_Destructure() = delete;
    Map_Destructure(const Map_Destructure&) = delete;
    Map_Destructure(Map_Destructure&&) = delete;
    Map_Destructure& operator=(const Map_Destructure&) = delete;
    Map_Destructure& operator=(Map_Destructure&&) = delete;
    ~Map_Destructure() final = default;

    Map_Destructure(std::vector<Element> destructure_elems,
                    Expression::Ptr expr, bool export_defs = false)
        : destructure_elems_{std::move(destructure_elems)}
        , expr_{std::move(expr)}
        , export_defs_{export_defs}
    {
        std::flat_set<std::string_view> binding_names;

        for (const auto& [_, name] : destructure_elems_)
        {
            if (binding_names.contains(name))
            {
                throw Frost_Unrecoverable_Error{fmt::format(
                    "Duplicate destructuring binding name: {}", name)};
            }

            binding_names.insert(name);
        }
    }

    std::optional<Map> execute(Symbol_Table& table) const final
    {
        Value_Ptr expr_result = expr_->evaluate(table);

        Map exports;

        auto define = [&](const std::string& name, const Value_Ptr& value) {
            table.define(name, value);
            if (export_defs_)
                exports.emplace(Value::create(auto{name}), value);
        };

        if (not expr_result->is<Map>())
        {
            throw Frost_Recoverable_Error{fmt::format(
                "Cannot destructure {} to Map", expr_result->type_name())};
        }

        const auto& map = expr_result->raw_get<Map>();

        for (const auto& [key_expr, name] : destructure_elems_)
        {
            auto key = key_expr->evaluate(table);
            if (!key->is_primitive())
            {
                throw Frost_Recoverable_Error{fmt::format(
                    "Non-primitive key expressions are not permitted in Map "
                    "destructuring: {}",
                    key->to_internal_string())};
            }
            auto itr = map.find(key);
            if (itr == map.end())
                define(name, Value::null());
            else
                define(name, itr->second);
        }

        if (export_defs_)
            return exports;
        else
            return std::nullopt;
    }

  protected:
    std::string node_label() const
    {
        return fmt::format("{}Map_Destructure", export_defs_ ? "Export_" : "");
    }

    std::generator<Child_Info> children() const
    {
        for (const auto& [key, name] : destructure_elems_)
            co_yield make_child(key, fmt::format("Bind({})", name));
        co_yield make_child(expr_, "RHS");
    }

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        co_yield std::ranges::elements_of(expr_->symbol_sequence());

        for (const auto& [key_expr, name] : destructure_elems_)
        {
            co_yield std::ranges::elements_of(key_expr->symbol_sequence());
            co_yield Definition{name};
        }
    }

  private:
    std::vector<Element> destructure_elems_;
    Expression::Ptr expr_;
    bool export_defs_;
};

} // namespace frst::ast

#endif
