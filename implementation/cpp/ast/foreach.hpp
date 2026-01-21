#ifndef FROST_AST_FOREACH_HPP
#define FROST_AST_FOREACH_HPP

#include "expression.hpp"
#include "frost/value.hpp"

namespace frst::ast
{

class Foreach final : public Expression
{
  public:
    using Ptr = std::unique_ptr<Foreach>;

    Foreach() = delete;
    Foreach(const Foreach&) = delete;
    Foreach(Foreach&&) = delete;
    Foreach& operator=(const Foreach&) = delete;
    Foreach& operator=(Foreach&&) = delete;
    ~Foreach() final = default;

    Foreach(Expression::Ptr structure, Expression::Ptr operation)
        : structure_{std::move(structure)}
        , operation_{std::move(operation)}
    {
    }

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final
    {
        const auto& structure_val = structure_->evaluate(syms);
        if (not structure_val->is_structured())
        {
            throw Frost_Recoverable_Error{
                fmt::format("Cannot iterate value with type {}",
                            structure_val->type_name())};
        }

        const auto& op_val = operation_->evaluate(syms);
        if (not op_val->is<Function>())
        {
            throw Frost_Recoverable_Error{
                fmt::format("Foreach operation expected Function, got {}",
                            op_val->type_name())};
        }

        const auto& op = op_val->raw_get<Function>();

        if (structure_val->is<Array>())
        {
            const auto& arr = structure_val->raw_get<Array>();
            for (const auto& elem : arr)
            {
                if (not op->call({elem})->as<Bool>().value())
                    break;
            }
            return Value::null();
        }

        if (structure_val->is<Map>())
        {
            const auto& map = structure_val->raw_get<Map>();
            for (const auto& [k, v] : map)
            {
                if (not op->call({k, v})->as<Bool>().value())
                    break;
            }
            return Value::null();
        }

        THROW_UNREACHABLE;
    }

  protected:
    std::generator<Child_Info> children() const final
    {
        co_yield make_child(structure_, "Structure");
        co_yield make_child(operation_, "Operation");
    }

    std::string node_label() const final
    {
        return "Foreach";
    }

  private:
    Expression::Ptr structure_;
    Expression::Ptr operation_;
};

} // namespace frst::ast

#endif
