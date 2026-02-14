#ifndef FROST_AST_FORMAT_STRING_HPP
#define FROST_AST_FORMAT_STRING_HPP

#include "expression.hpp"

#include <frost/utils.hpp>
#include <variant>

namespace frst::ast
{

class Format_String final : public Expression
{
  public:
    using Ptr = std::unique_ptr<Format_String>;

    Format_String() = delete;
    Format_String(const Format_String&) = delete;
    Format_String(Format_String&&) = delete;
    Format_String& operator=(const Format_String&) = delete;
    Format_String& operator=(Format_String&&) = delete;
    ~Format_String() final = default;

    Format_String(const std::string& format_string)
    {
        auto result = utils::parse_fmt_string(format_string);
        if (not result)
            throw Frost_Unrecoverable_Error{result.error()};
        segments_ = std::move(result).value();
    }

  public:
    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final
    {
        std::string out;
        for (const auto& segment : segments_)
        {
            segment.visit(Overload{
                [&](const utils::Fmt_Literal& literal) {
                    out.append(literal.text);
                },
                [&](const utils::Fmt_Placeholder& name) {
                    out.append(syms.lookup(name.text)->to_internal_string());
                }});
        }
        return Value::create(std::move(out));
    }

  protected:
    std::string node_label() const final
    {
        std::string reconstructed_string;

        for (const auto& segment : segments_)
            segment.visit(Overload{[&](const utils::Fmt_Literal& segment) {
                                       reconstructed_string.append(
                                           segment.text);
                                   },
                                   [&](const utils::Fmt_Placeholder& name) {
                                       reconstructed_string.append(
                                           fmt::format("${{{}}}", name.text));
                                   }});

        return fmt::format("Format_String({})", reconstructed_string);
    }

    std::generator<Symbol_Action> symbol_sequence() const final
    {
        for (const auto& segment : segments_)
        {
            if (std::holds_alternative<utils::Fmt_Placeholder>(segment))
                co_yield Usage{std::get<utils::Fmt_Placeholder>(segment).text};
        }
    }

  private:
    std::vector<utils::Fmt_Segment> segments_;
};

} // namespace frst::ast

#endif
