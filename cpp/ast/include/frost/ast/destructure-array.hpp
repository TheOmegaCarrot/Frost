#ifndef FROST_AST_DESTRUCTURE_ARRAY_HPP
#define FROST_AST_DESTRUCTURE_ARRAY_HPP

#include <frost/ast/destructure.hpp>

namespace frst::ast
{

class Destructure_Array final : public Destructure
{
  public:
    using Ptr = std::unique_ptr<Destructure_Array>;

    Destructure_Array() = delete;
    Destructure_Array(const Destructure_Array&) = delete;
    Destructure_Array(Destructure_Array&&) = delete;
    Destructure_Array& operator=(const Destructure_Array&) = delete;
    Destructure_Array& operator=(Destructure_Array&&) = delete;
    ~Destructure_Array() final = default;

    Destructure_Array(Source_Range source_range,
                      std::vector<Destructure::Ptr> destructures,
                      std::optional<std::string> rest_name, bool exported)
        : Destructure{source_range}
        , destructures_{std::move(destructures)}
        , rest_name_{std::move(rest_name)}
        , exported_{exported}
    {
    }

    std::generator<Symbol_Action> symbol_sequence() const
    {
        for (const auto& d : destructures_)
            co_yield std::ranges::elements_of(d->symbol_sequence());

        if (rest_name_ && rest_name_ != "_")
            co_yield Definition{rest_name_.value(), exported_};
    }

    std::generator<Child_Info> children() const final
    {
        for (const auto& d : destructures_)
            co_yield make_child(d);
    }

  protected:
    void do_destructure(Execution_Context ctx, const Value_Ptr& value) const
    {
        if (not value->is<Array>())
            throw Frost_Recoverable_Error{fmt::format(
                "Destructure expected Array, got {}", value->type_name())};

        const Array& arr = value->raw_get<Array>();

        if (arr.size() < destructures_.size())
        {
            throw Frost_Recoverable_Error{
                fmt::format("Insufficient Array elements to destructure: "
                            "required {} but got {}",
                            destructures_.size(), arr.size())};
        }

        if (not rest_name_ && arr.size() > destructures_.size())
        {
            throw Frost_Recoverable_Error{
                fmt::format("Too many Array elements to destructure: required "
                            "{} but got {}",
                            destructures_.size(), arr.size())};
        }

        for (const auto& [child, val] : std::views::zip(destructures_, arr))
            child->destructure(ctx, val);

        // permit `..._` as rest to discard rest
        if (rest_name_ == "_")
        {
            return;
        }
        else if (rest_name_)
        {
            ctx.symbols.define(
                rest_name_.value(),
                Value::create(arr
                              | std::views::drop(destructures_.size())
                              | std::ranges::to<Array>()));
        }
    }

    std::string do_node_label() const final
    {
        if (rest_name_)
            return fmt::format("Destructure_Array(rest: {})",
                               rest_name_.value());
        else
            return "Destructure_Array";
    }

  private:
    std::vector<Destructure::Ptr> destructures_;
    std::optional<std::string> rest_name_;
    bool exported_;
};

} // namespace frst::ast

#endif
