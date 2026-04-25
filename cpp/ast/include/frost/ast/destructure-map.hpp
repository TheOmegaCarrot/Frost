#ifndef FROST_AST_DESTRUCTURE_MAP_HPP
#define FROST_AST_DESTRUCTURE_MAP_HPP

#include <frost/ast/destructure.hpp>
#include <frost/ast/expression.hpp>

namespace frst::ast
{

class Destructure_Map final : public Destructure
{
  public:
    using Ptr = std::unique_ptr<Destructure_Map>;

    struct Element
    {
        Expression::Ptr key;
        Destructure::Ptr destructure;
    };

    Destructure_Map() = delete;
    Destructure_Map(const Destructure_Map&) = delete;
    Destructure_Map(Destructure_Map&&) = delete;
    Destructure_Map& operator=(const Destructure_Map&) = delete;
    Destructure_Map& operator=(Destructure_Map&&) = delete;
    ~Destructure_Map() final = default;

    Destructure_Map(const Source_Range& source_range,
                    std::vector<Element> destructure_elems,
                    std::optional<std::string> bind_whole_name, bool exported)
        : Destructure(source_range)
        , destructure_elems_{std::move(destructure_elems)}
        , bind_whole_name_{std::move(bind_whole_name)}
        , exported_{exported}
    {
    }

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        for (const auto& [key_expr, child] : destructure_elems_)
        {
            co_yield std::ranges::elements_of(key_expr->symbol_sequence());
            co_yield std::ranges::elements_of(child->symbol_sequence());
        }

        if (bind_whole_name_)
            co_yield Definition{.name = bind_whole_name_.value(),
                                .exported = exported_};
    }

    std::generator<Child_Info> children() const final
    {
        for (const auto& [i, pair] : std::views::enumerate(destructure_elems_))
        {
            // suboptimal but can be improved later on
            const auto& [key_expr, destructure_child] = pair;
            co_yield make_child(key_expr, fmt::format("Key {}", i + 1));
            co_yield make_child(destructure_child,
                                fmt::format("Binding {}", i + 1));
        }
    }

  protected:
    std::string do_node_label() const final
    {
        return fmt::format(
            "Destructure_Map{}",
            bind_whole_name_ ? fmt::format("(as {})", bind_whole_name_.value())
                             : "");
    }

    void do_destructure(Execution_Context ctx,
                        const Value_Ptr& value) const final;

  private:
    std::vector<Element> destructure_elems_;
    std::optional<std::string> bind_whole_name_;
    bool exported_;
};

} // namespace frst::ast

#endif
