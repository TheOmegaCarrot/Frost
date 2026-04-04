#include <frost/ast/define.hpp>

using namespace frst;

ast::Define::Define(Source_Range source_range, Destructure::Ptr destructure,
                    Expression::Ptr expr)
    : Statement(source_range)
    , destructure_{std::move(destructure)}
    , expr_{std::move(expr)}
{
}

void ast::Define::do_execute(Execution_Context& ctx) const
{
    auto value = expr_->evaluate(ctx.as_eval());
    destructure_->destructure(ctx, value);
}

std::generator<ast::AST_Node::Symbol_Action> ast::Define::symbol_sequence()
    const
{
    co_yield std::ranges::elements_of(expr_->symbol_sequence());
    co_yield std::ranges::elements_of(destructure_->symbol_sequence());
}

std::string ast::Define::do_node_label() const
{
    return fmt::format("Define");
}

std::generator<ast::AST_Node::Child_Info> ast::Define::children() const
{
    co_yield make_child(expr_, "Expression");
    co_yield make_child(destructure_, "Bindings");
}
