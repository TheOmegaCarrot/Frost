#include <frost/ast/do.hpp>
#include <frost/ast/utils/block-utils.hpp>

#include <flat_set>

using namespace frst;
using namespace frst::ast;

namespace
{
template <typename To, typename From>
std::unique_ptr<To> unique_dynamic_cast(std::unique_ptr<From>& from)
{
    if (auto* raw = dynamic_cast<To*>(from.get()))
    {
        from.release();
        return std::unique_ptr<To>{raw};
    }
    return nullptr;
}
} // namespace

Do_Block::Do_Block(std::vector<ast::Statement::Ptr> body)
    : body_prefix_{std::move(body)}
{
    if (body_prefix_.size() == 0)
        throw Frost_Unrecoverable_Error{"Do block may not have an empty body"};

    ast::Statement::Ptr last_statement = std::move(body_prefix_.back());
    body_prefix_.pop_back();

    value_expr_ = unique_dynamic_cast<ast::Expression>(last_statement);
    if (not value_expr_)
        throw Frost_Unrecoverable_Error{"A do block must end in an expression"};
}

Value_Ptr Do_Block::do_evaluate(const Symbol_Table& syms) const
{
    Symbol_Table exec_table{&syms};
    for (const auto& statement : body_prefix_)
        statement->execute(exec_table);
    return value_expr_->evaluate(exec_table);
}

std::string Do_Block::node_label() const
{
    return "Do_Block";
}

std::generator<Statement::Child_Info> Do_Block::children() const
{
    for (const auto& statement : body_prefix_)
        co_yield make_child(statement);
    co_yield make_child(value_expr_);
}

std::generator<Statement::Symbol_Action> Do_Block::symbol_sequence() const
{
    std::flat_set<std::string> defns;

    const auto get_name = [](const auto& action) -> const auto& {
        return action.name;
    };

    for (const Statement::Symbol_Action& action :
         utils::body_symbol_sequence(body_prefix_, value_expr_))
    {
        const auto name = action.visit(get_name);
        if (std::holds_alternative<Statement::Definition>(action))
            defns.insert(name);
        else if (not defns.contains(name))
            co_yield action;
    }
}
