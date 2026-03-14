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

    Name_Lookup(Source_Range source_range, std::string name);

    std::generator<Symbol_Action> symbol_sequence() const final;

  protected:
    std::string do_node_label() const final;

    [[nodiscard]] Value_Ptr do_evaluate(Evaluation_Context ctx) const final;

  private:
    std::string name_;
};
} // namespace frst::ast

#endif
