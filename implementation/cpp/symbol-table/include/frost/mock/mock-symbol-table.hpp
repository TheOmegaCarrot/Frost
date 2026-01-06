#ifndef FROST_AST_MOCK_SYMBOL_TABLE_HPP
#define FROST_AST_MOCK_SYMBOL_TABLE_HPP

#include <trompeloeil.hpp>

#include <frost/symbol-table.hpp>

namespace frst::mock
{
class Mock_Symbol_Table : public Symbol_Table
{
    MAKE_MOCK(define, auto(std::string, Value_Ptr)->void, override);

    MAKE_CONST_MOCK(lookup, auto(const std::string&)->Value_Ptr, override);
};
} // namespace frst::mock

#endif
