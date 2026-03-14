#include <frost/ast/array-constructor.hpp>

using namespace frst;

ast::Array_Constructor::Array_Constructor(std::vector<Expression::Ptr> elems)
    : elems_{std::move(elems)}
{
}

Value_Ptr ast::Array_Constructor::do_evaluate(Evaluation_Context ctx) const
{
    return Value::create(elems_
                         | std::views::transform([&](const auto& each) {
                               return each->evaluate(ctx);
                           })
                         | std::ranges::to<std::vector>());
}

std::string ast::Array_Constructor::node_label() const
{
    return "Array_Constructor";
}

std::generator<ast::Statement::Child_Info> ast::Array_Constructor::children()
    const
{
    for (const auto& elem : elems_)
        co_yield make_child(elem);
}

bool ast::Array_Constructor::data_safe() const
{
    return true;
}
