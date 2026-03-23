#ifndef FROST_AST_ARRAY_DESTRUCTURE_HPP
#define FROST_AST_ARRAY_DESTRUCTURE_HPP

#include "expression.hpp"
#include "statement.hpp"

#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

namespace frst::ast
{

struct Discarded_Binding
{
    constexpr static std::string_view token = "_";
};

class Array_Destructure final : public Statement
{
  public:
    using Ptr = std::unique_ptr<Array_Destructure>;
    using Name = std::variant<Discarded_Binding, std::string>;

    Array_Destructure() = delete;
    Array_Destructure(const Array_Destructure&) = delete;
    Array_Destructure(Array_Destructure&&) = delete;
    Array_Destructure& operator=(const Array_Destructure&) = delete;
    Array_Destructure& operator=(Array_Destructure&&) = delete;
    ~Array_Destructure() final = default;

    Array_Destructure(Source_Range source_range, std::vector<Name> names,
                      std::optional<Name> rest_name, Expression::Ptr expr,
                      bool export_defs = false);

    std::generator<Symbol_Action> symbol_sequence() const final;

  protected:
    std::string do_node_label() const final;

    void do_execute(Execution_Context& ctx) const final;

    std::generator<Child_Info> children() const;

  private:
    std::vector<Name> names_;
    std::optional<Name> rest_name_;
    Expression::Ptr expr_;
    bool export_defs_;
};

} // namespace frst::ast

#endif
