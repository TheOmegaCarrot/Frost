#ifndef FROST_CLOSURE_HPP
#define FROST_CLOSURE_HPP

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

#include <sstream>

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

    Value_Ptr call(const std::vector<Value_Ptr>& args) const override;
    std::string debug_dump() const override
    {
        std::ostringstream os;
        os << "<Closure>";
        for (const auto& statement : body_)
        {
            os << "    ";
            statement->debug_dump_ast(os);
        }

        return std::move(os).str();
    }

  private:
    std::vector<ast::Statement::Ptr> body_;
    Symbol_Table captures_;
};
} // namespace frst

#endif
