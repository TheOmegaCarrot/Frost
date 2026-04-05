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

    Destructure_Map(Source_Range source_range,
                    std::vector<Element> destructure_elems)
        : Destructure(source_range)
        , destructure_elems_{std::move(destructure_elems)}
    {
    }

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        for (const auto& [key_expr, child] : destructure_elems_)
        {
            co_yield std::ranges::elements_of(key_expr->symbol_sequence());
            co_yield std::ranges::elements_of(child->symbol_sequence());
        }
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

    std::string do_node_label() const final
    {
        return "Destructure_Map";
    }

    void do_destructure(Execution_Context ctx,
                        const Value_Ptr& value) const final
    {
        if (not value->is<frst::Map>())
            throw Frost_Recoverable_Error{fmt::format(
                "Destructure expected Map, got {}", value->type_name())};

        const frst::Map& map_being_destructured = value->raw_get<frst::Map>();

        for (const auto& [key_expr, destructure_child] : destructure_elems_)
        {
            auto key = key_expr->evaluate(ctx.as_eval());
            if (not key->is_primitive() || key->is<Null>())
            {
                throw Frost_Recoverable_Error{
                    fmt::format("Map destructure key expressions must be valid "
                                "Map keys, got: {}",
                                key->type_name())};
            }

            auto itr = map_being_destructured.find(key);
            if (itr == map_being_destructured.end())
                destructure_child->destructure(ctx, Value::null());
            else
                destructure_child->destructure(ctx, itr->second);
        }
    }

  protected:
  private:
    std::vector<Element> destructure_elems_;
};

} // namespace frst::ast

#endif
