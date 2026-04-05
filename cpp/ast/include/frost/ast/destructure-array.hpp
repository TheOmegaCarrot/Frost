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

    Destructure_Array(const Source_Range& source_range,
                      std::vector<Destructure::Ptr> destructures,
                      std::optional<std::string> rest_name, bool exported)
        : Destructure{source_range}
        , destructures_{std::move(destructures)}
        , rest_name_{std::move(rest_name)}
        , exported_{exported}
    {
    }

    std::generator<Symbol_Action> symbol_sequence() const final
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
    void do_destructure(Execution_Context ctx,
                        const Value_Ptr& value) const final;

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
