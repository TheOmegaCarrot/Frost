#ifndef FROST_EXECUTION_CONTEXT_HPP
#define FROST_EXECUTION_CONTEXT_HPP

#include "symbol-table.hpp"

namespace frst
{

struct Evaluation_Context
{
    const Symbol_Table& symbols;
};

struct Execution_Context
{
    Symbol_Table& symbols;

    Evaluation_Context as_eval() const
    {
        return {.symbols = symbols};
    }
};

} // namespace frst

#endif
