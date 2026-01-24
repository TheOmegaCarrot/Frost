#ifndef FROST_AST_NAME_LOOKUP_HPP
#define FROST_AST_NAME_LOOKUP_HPP

#include "expression.hpp"

#include <fmt/format.h>

namespace frst::ast
{

class Name_Lookup final : public Expression
{
  public:
    using Ptr = std::unique_ptr<Name_Lookup>;

    Name_Lookup() = delete;
    Name_Lookup(const Name_Lookup&) = delete;
    Name_Lookup(Name_Lookup&&) = delete;
    Name_Lookup& operator=(const Name_Lookup&) = delete;
    Name_Lookup& operator=(Name_Lookup&&) = delete;
    ~Name_Lookup() final = default;

    Name_Lookup(std::string name)
        : name_{std::move(name)}
    {
        if (name_ == "_")
            throw Frost_Unrecoverable_Error{"\"_\" is not a valid identifier"};
    }

  public:
    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final
    {
        return syms.lookup(name_);
    }

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        co_yield Usage{name_};
    }

  protected:
    std::string node_label() const final
    {
        return fmt::format("Name_Lookup({})", name_);
    }

  private:
    std::string name_;
};
} // namespace frst::ast

#endif
