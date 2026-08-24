#ifndef FROST_CLOSURE_HPP
#define FROST_CLOSURE_HPP

#include <frost/ast/expression.hpp>
#include <frost/ast/statement.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <memory>

namespace frst
{
class Closure : public Callable, public std::enable_shared_from_this<Closure>
{
  public:
    Closure() = delete;
    Closure(const Closure&) = delete;
    Closure(Closure&&) = delete;
    Closure& operator=(const Closure&) = delete;
    Closure& operator=(Closure&&) = delete;
    ~Closure() override = default;

    Closure(std::vector<std::string> parameters,
            std::shared_ptr<std::vector<ast::Statement::Ptr>> body,
            std::shared_ptr<ast::Expression> return_expr, Symbol_Table captures,
            std::size_t define_count,
            std::optional<std::string> vararg_parameter = {},
            std::optional<std::string> self_name = {});

    static std::shared_ptr<Closure> create(
        std::vector<std::string> parameters,
        std::shared_ptr<std::vector<ast::Statement::Ptr>> body,
        std::shared_ptr<ast::Expression> return_expr, Symbol_Table captures,
        std::size_t define_count,
        std::optional<std::string> vararg_parameter = {},
        std::optional<std::string> self_name = {});

    Value_Ptr call(std::span<const Value_Ptr> args) const override;
    std::string debug_dump() const override;
    std::string name() const override;
    const Symbol_Table& debug_capture_table() const;

  private:
    Function self_function() const;

    std::vector<std::string> parameters_;
    std::shared_ptr<std::vector<ast::Statement::Ptr>> body_prefix_;
    std::shared_ptr<ast::Expression> return_expr_;
    Symbol_Table captures_;
    std::optional<std::string> vararg_parameter_;
    std::optional<std::string> self_name_;
    std::size_t define_count_;
};
} // namespace frst

#endif
