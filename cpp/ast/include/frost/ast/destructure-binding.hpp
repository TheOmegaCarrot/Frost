#ifndef FROST_AST_DESTRUCTURE_BINDING_HPP
#define FROST_AST_DESTRUCTURE_BINDING_HPP

#include "frost/execution-context.hpp"
#include <frost/ast/destructure.hpp>

namespace frst::ast
{

class Destructure_Binding final : public Destructure
{
  public:
    using Ptr = std::unique_ptr<Destructure_Binding>;

    Destructure_Binding(const Source_Range& source_range,
                        std::optional<std::string> name, bool exported)
        : Destructure(source_range)
        , name_{std::move(name)}
        , exported_{exported}
    {
        if (name_.has_value() && name_.value().empty())
            throw Frost_Interpreter_Error{"Attempting to bind an empty name!"};
        if (name_.has_value())
            forbid_dollar_identifier(name_.value());
    }

    Destructure_Binding() = delete;
    Destructure_Binding(const Destructure_Binding&) = delete;
    Destructure_Binding(Destructure_Binding&&) = delete;
    Destructure_Binding& operator=(const Destructure_Binding&) = delete;
    Destructure_Binding& operator=(Destructure_Binding&&) = delete;
    ~Destructure_Binding() final = default;

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
        return fmt::format("Destructure_Binding({})",
                           name_.value_or("discarded"));
    }

  private:
    std::optional<std::string> name_;
    bool exported_;
};

} // namespace frst::ast

#endif
