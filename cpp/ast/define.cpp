#include <frost/ast/define.hpp>

using namespace frst;

ast::Define::Define(Source_Range source_range, std::string name,
                    Expression::Ptr expr, bool export_def)
    : Statement(source_range)
    , name_{std::move(name)}
    , expr_{std::move(expr)}
    , export_def_{export_def}
{
    if (name_ == "_")
        throw Frost_Unrecoverable_Error{"\"_\" is not a valid identifier"};
}

void ast::Define::do_execute(Execution_Context& ctx) const
{
    auto value = expr_->evaluate(ctx.as_eval());
    ctx.symbols.define(name_, value);
}

std::generator<ast::Statement::Symbol_Action> ast::Define::symbol_sequence()
    const
{
    co_yield std::ranges::elements_of(expr_->symbol_sequence());
    co_yield Definition{.name = name_, .exported = export_def_};
}

std::string ast::Define::do_node_label() const
{
    return fmt::format("{}Define({})", export_def_ ? "Export_" : "", name_);
}

std::generator<ast::Statement::Child_Info> ast::Define::children() const
{
    co_yield make_child(expr_);
}
