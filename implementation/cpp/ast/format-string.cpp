#include <frost/ast/format-string.hpp>

using namespace frst;

ast::Format_String::Format_String(const std::string& format_string)
{
    auto result = utils::parse_fmt_string(format_string);
    if (not result)
        throw Frost_Unrecoverable_Error{result.error()};
    segments_ = std::move(result).value();
}

Value_Ptr ast::Format_String::evaluate(const Symbol_Table& syms) const
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

std::string ast::Format_String::node_label() const
{
    std::string reconstructed_string;

    for (const auto& segment : segments_)
        segment.visit(Overload{[&](const utils::Fmt_Literal& segment) {
                                   reconstructed_string.append(segment.text);
                               },
                               [&](const utils::Fmt_Placeholder& name) {
                                   reconstructed_string.append(
                                       fmt::format("${{{}}}", name.text));
                               }});

    return fmt::format("Format_String({})", reconstructed_string);
}

auto ast::Format_String::symbol_sequence() const
    -> std::generator<Symbol_Action>
{
    for (const auto& segment : segments_)
    {
        if (std::holds_alternative<utils::Fmt_Placeholder>(segment))
            co_yield Usage{std::get<utils::Fmt_Placeholder>(segment).text};
    }
}
