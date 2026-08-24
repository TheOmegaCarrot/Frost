#ifndef FROST_AST_EXPR_FORMAT_STRING_HPP
#define FROST_AST_EXPR_FORMAT_STRING_HPP

#include <frost/ast/expression.hpp>

#include <variant>

namespace frst::ast
{

// Format string with expression interpolation support.
// Stores a sequence of segments: either literal text or an expression
// to evaluate and convert to string.
class Format_String final : public Expression
{
  public:
    using Ptr = std::unique_ptr<Format_String>;

    struct Literal_Segment
    {
        std::string text;
    };

    using Segment = std::variant<Literal_Segment, Expression::Ptr>;

    Format_String(const Source_Range& source_range,
                  std::vector<Segment> segments)
        : Expression(source_range)
        , segments_{std::move(segments)}
    {
    }

    Format_String() = delete;
    Format_String(const Format_String&) = delete;
    Format_String(Format_String&&) = delete;
    Format_String& operator=(const Format_String&) = delete;
    Format_String& operator=(Format_String&&) = delete;
    ~Format_String() final = default;

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        for (const auto& seg : segments_)
            if (const auto* expr = std::get_if<Expression::Ptr>(&seg))
                co_yield std::ranges::elements_of((*expr)->symbol_sequence());
    }

    std::generator<Child_Info> children() const final
    {
        for (const auto& [i, seg] : std::views::enumerate(segments_))
            if (const auto* expr = std::get_if<Expression::Ptr>(&seg))
                co_yield make_child(*expr, fmt::format("Interpolation {}", i));
    }

  protected:
    std::string do_node_label() const final
    {
        return "Format_String";
    }

    [[nodiscard]] Value_Ptr do_evaluate(Evaluation_Context ctx) const final
    {
        std::string result;
        for (const auto& seg : segments_)
        {
            if (const auto* lit = std::get_if<Literal_Segment>(&seg))
                result += lit->text;
            else
                result += std::get<Expression::Ptr>(seg)
                              ->evaluate(ctx)
                              ->to_internal_string();
        }
        return Value::create(std::move(result));
    }

  private:
    std::vector<Segment> segments_;
};

} // namespace frst::ast

#endif
