#ifndef FROST_CLOSURE_HPP
#define FROST_CLOSURE_HPP

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

namespace frst
{
class Closure : public Callable
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
            Symbol_Table captures,
            std::optional<std::string> vararg_parameter = {});

    Value_Ptr call(std::span<const Value_Ptr> args) const override;
    std::string debug_dump() const override;
    const Symbol_Table& debug_capture_table() const
    {
        return captures_;
    }

    void inject_capture(const std::string& name, Value_Ptr value)
    {
        captures_.define(name, value);
    }

  private:
    std::vector<std::string> parameters_;
    std::shared_ptr<std::vector<ast::Statement::Ptr>> body_;
    Symbol_Table captures_;
    std::optional<std::string> vararg_parameter_;
};

class Weak_Closure final : public Callable
{
  public:
    Weak_Closure() = delete;
    Weak_Closure(const Weak_Closure&) = delete;
    Weak_Closure(Weak_Closure&&) = delete;
    Weak_Closure& operator=(const Weak_Closure&) = delete;
    Weak_Closure& operator=(Weak_Closure&&) = delete;
    ~Weak_Closure() override = default;

    Weak_Closure(std::weak_ptr<Closure> closure)
        : closure_{closure}
    {
    }

    Value_Ptr call(std::span<const Value_Ptr> args) const final
    {
        auto closure = closure_.lock();
        if (!closure)
            throw Frost_Internal_Error{"Closure self-reference expired"};
        return closure->call(args);
    }

    std::string debug_dump() const override
    {
        return "<closure self-reference>";
    }

  private:
    std::weak_ptr<Closure> closure_;
};
} // namespace frst

#endif
