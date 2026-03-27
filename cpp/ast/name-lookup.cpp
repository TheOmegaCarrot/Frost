#include <frost/ast/name-lookup.hpp>

using namespace frst;

ast::Name_Lookup::Name_Lookup(Source_Range source_range, std::string name)
    : Expression(source_range)
    , name_{std::move(name)}
{
    if (name_ == "_")
        throw Frost_Unrecoverable_Error{"\"_\" is not a valid identifier"};
}

Value_Ptr ast::Name_Lookup::do_evaluate(Evaluation_Context ctx) const
{
    return ctx.symbols.lookup(name_);
}

std::generator<ast::AST_Node::Symbol_Action> ast::Name_Lookup::
    symbol_sequence() const
{
    co_yield Usage{name_};
}

std::string ast::Name_Lookup::do_node_label() const
{
    return fmt::format("Name_Lookup({})", name_);
}
