#ifndef FROST_AST_UTILS_BLOCK_UTILS_HPP
#define FROST_AST_UTILS_BLOCK_UTILS_HPP

#include <frost/ast/expression.hpp>

namespace frst::ast::utils
{

struct
{
    static std::generator<Statement::Symbol_Action> operator()(
        const auto& node_ptr)
    {
        return node_ptr->symbol_sequence();
    }
} constexpr static node_to_sym_seq;

template <std::indirectly_readable Node_Ptr>
auto body_symbol_sequence(const std::vector<Statement::Ptr>& body_prefix,
                          const Node_Ptr& return_expr)
{
    return std::views::concat(
        body_prefix | std::views::transform(node_to_sym_seq) | std::views::join,
        node_to_sym_seq(return_expr));
}

} // namespace frst::ast::utils

#endif
