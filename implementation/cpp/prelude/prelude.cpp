#include <frost/parser.hpp>
#include <frost/prelude.hpp>

namespace frst
{

constexpr static const char prelude_text[]{
#embed "prelude.frst"
};

void inject_prelude(Symbol_Table& table)
{
    auto ast = parse_program(prelude_text);
    for (const auto& statement : ast.value())
        statement->execute(table);
}

} // namespace frst
