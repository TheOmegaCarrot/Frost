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
            std::vector<ast::Statement::Ptr>* body,
            const Symbol_Table& construction_environment);

    Value_Ptr call(const std::vector<Value_Ptr>& args) const override;
    std::string debug_dump() const override;
    const Symbol_Table& debug_capture_table() const
    {
        return captures_;
    }

  private:
    std::vector<std::string> parameters_;
    std::vector<ast::Statement::Ptr>* body_;
    Symbol_Table captures_;
};
} // namespace frst

#endif
