#include <frost/builtins-common.hpp>
#include <frost/meta.hpp>
#include <frost/parser.hpp>

namespace frst
{

BUILTIN(read_value)
{
    REQUIRE_ARGS("read_value", PARAM("data", TYPES(String)));

    const auto& data = GET(0, String);
    auto parse_result = parse_data(data);

    if (not parse_result)
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Invalid Frost Data:\nError: {}", parse_result.error())};
    }

    const auto& expr = parse_result.value();

    for (const auto* node : expr->walk())
    {
        if (not node->data_safe())
            throw Frost_Recoverable_Error{fmt::format(
                "Invalid node in Frost Data: {}", node->node_label())};
    }

    // no data safe node accesses the symbol table,
    // so it's fine for it to be empty
    Symbol_Table empty;
    return expr->evaluate({.symbols = empty});
}

void inject_meta(Symbol_Table& table)
{
    INJECT(read_value);
}
} // namespace frst
