#ifndef FROST_EXECUTION_CONTEXT_HPP
#define FROST_EXECUTION_CONTEXT_HPP

#include "symbol-table.hpp"

namespace frst
{

struct Execution_Context
{
    Symbol_Table& symbols;
};

} // namespace frst

#endif
