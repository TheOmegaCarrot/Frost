#ifndef FROST_AST_MAP_DESTRUCTURE_HPP
#define FROST_AST_MAP_DESTRUCTURE_HPP

#include "expression.hpp"
#include "frost/value.hpp"
#include "statement.hpp"

#include <frost/symbol-table.hpp>

namespace frst::ast
{

class Map_Destructure final : public Statement
{

  public:
    using Ptr = std::unique_ptr<Map_Destructure>;

    struct Element // { [key]: name }
    {
        Expression::Ptr key;
        std::string name;
    };

    Map_Destructure() = delete;
    Map_Destructure(const Map_Destructure&) = delete;
    Map_Destructure(Map_Destructure&&) = delete;
    Map_Destructure& operator=(const Map_Destructure&) = delete;
    Map_Destructure& operator=(Map_Destructure&&) = delete;
    ~Map_Destructure() final = default;

    Map_Destructure(std::vector<Element> destructure_elems,
                    Expression::Ptr expr, bool export_defs = false);

    std::optional<Map> execute(Symbol_Table& table) const final;

  protected:
    std::string node_label() const;

    std::generator<Child_Info> children() const;

    std::generator<Symbol_Action> symbol_sequence() const final;

  private:
    std::vector<Element> destructure_elems_;
    Expression::Ptr expr_;
    bool export_defs_;
};

} // namespace frst::ast

#endif
