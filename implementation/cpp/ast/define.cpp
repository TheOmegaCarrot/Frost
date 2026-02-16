#include <frost/ast/define.hpp>

using namespace frst;

ast::Define::Define(std::string name, Expression::Ptr expr, bool export_def)
    : name_{std::move(name)}
    , expr_{std::move(expr)}
    , export_def_{export_def}
{
    if (name_ == "_")
        throw Frost_Unrecoverable_Error{"\"_\" is not a valid identifier"};
}

std::optional<Map> ast::Define::execute(Symbol_Table& table) const
{
    auto value = expr_->evaluate(table);
    table.define(name_, value);

    if (export_def_)
        return Map{{Value::create(auto{name_}), value}};
    else
        return std::nullopt;
}

auto ast::Define::symbol_sequence() const -> std::generator<Symbol_Action>
{
    co_yield std::ranges::elements_of(expr_->symbol_sequence());
    co_yield Definition{name_};
}

std::string ast::Define::node_label() const
{
    return fmt::format("{}Define({})", export_def_ ? "Export_" : "", name_);
}

auto ast::Define::children() const -> std::generator<Child_Info>
{
    co_yield make_child(expr_);
}
