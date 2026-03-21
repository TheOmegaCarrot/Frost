#ifndef FROST_PRELUDE_HPP
#define FROST_PRELUDE_HPP

#include <frost/execution-context.hpp>

namespace frst
{

void inject_prelude(Symbol_Table& table);
void inject_prelude(Execution_Context ctx);

}

#endif
