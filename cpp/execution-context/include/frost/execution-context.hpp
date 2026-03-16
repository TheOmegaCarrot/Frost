#ifndef FROST_EXECUTION_CONTEXT_HPP
#define FROST_EXECUTION_CONTEXT_HPP

#include "symbol-table.hpp"

namespace frst
{

class Backtrace_State;

struct Runtime_Context
{
    Backtrace_State* backtrace = nullptr;
};

struct Evaluation_Context
{
    const Symbol_Table& symbols;
    Runtime_Context runtime;
};

struct Execution_Context
{
    Symbol_Table& symbols;
    Runtime_Context runtime;

    Evaluation_Context as_eval() const
    {
        return {.symbols = symbols, .runtime = runtime};
    }
};

} // namespace frst

#endif
