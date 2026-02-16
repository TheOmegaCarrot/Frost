#include <frost/ast/name_lookup.hpp>

using namespace frst;

ast::Name_Lookup::Name_Lookup(std::string name)
    : name_{std::move(name)}
{
    if (name_ == "_")
        throw Frost_Unrecoverable_Error{"\"_\" is not a valid identifier"};
}

Value_Ptr ast::Name_Lookup::evaluate(const Symbol_Table& syms) const
{
    return syms.lookup(name_);
}

auto ast::Name_Lookup::symbol_sequence() const -> std::generator<Symbol_Action>
{
    co_yield Usage{name_};
}

std::string ast::Name_Lookup::node_label() const
{
    return fmt::format("Name_Lookup({})", name_);
}
