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

    Format_String(const std::string& format_string);

  public:
    [[nodiscard]] Value_Ptr evaluate(const Symbol_Table& syms) const final;

  protected:
    std::string node_label() const final;

    std::generator<Symbol_Action> symbol_sequence() const final;

  private:
    std::vector<utils::Fmt_Segment> segments_;
};

} // namespace frst::ast

#endif
