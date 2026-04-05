#ifndef FROST_AST_DESTRUCTURE_LEAF_HPP
#define FROST_AST_DESTRUCTURE_LEAF_HPP

#include "frost/execution-context.hpp"
#include <frost/ast/destructure.hpp>

namespace frst::ast
{

class Destructure_Leaf final : public Destructure
{
  public:
    using Ptr = std::unique_ptr<Destructure_Leaf>;

    Destructure_Leaf(const Source_Range& source_range, std::optional<std::string> name,
                     bool exported)
        : Destructure(source_range)
        , name_{std::move(name)}
        , exported_{exported}
    {
        if (name_.has_value() && name_.value().empty())
            throw Frost_Interpreter_Error{"Attempting to bind an empty name!"};
    }

    Destructure_Leaf() = delete;
    Destructure_Leaf(const Destructure_Leaf&) = delete;
    Destructure_Leaf(Destructure_Leaf&&) = delete;
    Destructure_Leaf& operator=(const Destructure_Leaf&) = delete;
    Destructure_Leaf& operator=(Destructure_Leaf&&) = delete;
    ~Destructure_Leaf() final = default;

    std::generator<AST_Node::Symbol_Action> symbol_sequence() const final
    {
        if (name_)
            co_yield AST_Node::Definition{name_.value(), exported_};
    }

  protected:
    void do_destructure(Execution_Context ctx,
                        const Value_Ptr& value) const final
    {
        if (name_)
            ctx.symbols.define(name_.value(), value);
    }

    std::string do_node_label() const final
    {
        return fmt::format("Destructure_Leaf({})", name_.value_or("discarded"));
    }

  private:
    std::optional<std::string> name_;
    bool exported_;
};

} // namespace frst::ast

#endif
