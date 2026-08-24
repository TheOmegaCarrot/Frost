#include <frost/parser.hpp>
#include <frost/prelude.hpp>

namespace frst
{

constexpr static const char prelude_text[]{
#embed "prelude.frst"
};

void inject_prelude(Symbol_Table& table)
{
    Execution_Context ctx{.symbols = table};
    inject_prelude(ctx);
}

void inject_prelude(Execution_Context ctx)
{
    auto ast = parse_program(prelude_text, "<prelude>");

    if (!ast)
        throw Frost_Interpreter_Error{ast.error()};

    for (const auto& statement : ast.value())
        statement->execute(ctx);
}

} // namespace frst
