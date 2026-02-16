#include <frost/ast/array-constructor.hpp>

using namespace frst;

ast::Array_Constructor::Array_Constructor(std::vector<Expression::Ptr> elems)
    : elems_{std::move(elems)}
{
}

Value_Ptr ast::Array_Constructor::evaluate(const Symbol_Table& syms) const
{
    return Value::create(elems_
                         | std::views::transform([&](const auto& each) {
                               return each->evaluate(syms);
                           })
                         | std::ranges::to<std::vector>());
}

std::string ast::Array_Constructor::node_label() const
{
    return "Array_Constructor";
}

std::generator<ast::Statement::Child_Info> ast::Array_Constructor::children() const
{
    for (const auto& elem : elems_)
        co_yield make_child(elem);
}
