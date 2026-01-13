#ifndef FROST_AST_BINOP_HPP
#define FROST_AST_BINOP_HPP

#include "expression.hpp"
#include "frost/symbol-table.hpp"

#include <fmt/format.h>

#include <frost/value.hpp>

namespace frst::ast
{

enum class Binary_Op
{
    PLUS,
    MINUS,
    TIMES,
    DIVIDE,
    EQ,
    NE,
    LT,
    LE,
    GT,
    GE,
    AND,
    OR,
};

struct Convert_Binary_Op
{
    using enum Binary_Op;
    static std::optional<Binary_Op> operator()(const std::string& op)
    {
        if (op == "+")
            return PLUS;
        if (op == "-")
            return MINUS;
        if (op == "*")
            return TIMES;
        if (op == "/")
            return DIVIDE;
        if (op == "==")
            return EQ;
        if (op == "!=")
            return NE;
        if (op == "<")
            return LT;
        if (op == "<=")
            return LE;
        if (op == ">")
            return GT;
        if (op == ">=")
            return GE;
        if (op == "and")
            return AND;
        if (op == "or")
            return OR;

        return std::nullopt;
    }
    static std::string_view operator()(Binary_Op op)
    {
        switch (op)
        {
        case PLUS:
            return "+";
        case MINUS:
            return "-";
        case TIMES:
            return "*";
        case DIVIDE:
            return "/";
        case EQ:
            return "==";
        case NE:
            return "!=";
        case LT:
            return "<";
        case LE:
            return "<=";
        case GT:
            return ">";
        case GE:
            return ">=";
        case AND:
            return "and";
        case OR:
            return "or";
        }
        THROW_UNREACHABLE;
    }
} constexpr static convert_binary_op;

class Binop final : public Expression
{

  public:
    using Ptr = std::unique_ptr<Binop>;

    Binop(Expression::Ptr lhs, const std::string& op, Expression::Ptr rhs)
        : lhs_{std::move(lhs)}
        , rhs_{std::move(rhs)}
        , op_{convert_binary_op(op)
                  .or_else([&] -> std::optional<Binary_Op> {
                      throw Frost_Error{
                          fmt::format("Bad binary operator {}", op)};
                      return {};
                  })
                  .value()}
    {
    }

    Binop() = delete;
    Binop(const Binop&) = delete;
    Binop(Binop&&) = delete;
    Binop& operator=(const Binop&) = delete;
    Binop& operator=(Binop&&) = delete;
    ~Binop() override = default;

    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final
    {
        using enum Binary_Op;
        static const std::map<Binary_Op, decltype(&Value::add)> fn_map{
            {PLUS, &Value::add},        {MINUS, &Value::subtract},
            {TIMES, &Value::multiply},  {DIVIDE, &Value::divide},
            {EQ, &Value::equal},        {NE, &Value::not_equal},
            {LT, &Value::less_than},    {LE, &Value::less_than_or_equal},
            {GT, &Value::greater_than}, {GE, &Value::greater_than_or_equal},
        };

        auto lhs_val = lhs_->evaluate(syms);

        if (auto itr = fn_map.find(op_); itr != fn_map.end())
        {
            auto rhs_val = rhs_->evaluate(syms);
            return itr->second(lhs_val, rhs_val);
        }

        if (op_ == AND)
        {
            if (lhs_val->as<Bool>().value())
                return rhs_->evaluate(syms);
            else
                return lhs_val;
        }

        if (op_ == OR)
        {
            if (lhs_val->as<Bool>().value())
                return lhs_val;
            else
                return rhs_->evaluate(syms);
        }

        THROW_UNREACHABLE;
    }

  protected:
    std::string node_label() const final
    {
        return fmt::format("Binary({})", convert_binary_op(op_));
    }

    std::generator<Child_Info> children() const final
    {
        co_yield make_child(lhs_, "LHS");
        co_yield make_child(rhs_, "RHS");
    }

  private:
    Expression::Ptr lhs_;
    Expression::Ptr rhs_;
    Binary_Op op_;
};
} // namespace frst::ast

#endif
