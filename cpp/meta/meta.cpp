#include <frost/builtins-common.hpp>
#include <frost/meta.hpp>
#include <frost/parser.hpp>

namespace frst
{

BUILTIN(read_data)
{
    REQUIRE_ARGS("read_value", PARAM("data", TYPES(String)));

    const auto& data = GET(0, String);
    auto parse_result = parse_data(data);

    if (not parse_result)
    {
        throw Frost_Recoverable_Error{fmt::format(
            "Invalid Frost Data:\n{}\nError: {}", data, parse_result.error())};
    }

    const auto& ast = parse_result.value();

    const auto* expr = dynamic_cast<ast::Expression*>(ast.get());

    if (not expr)
        throw Frost_Recoverable_Error{fmt::format(
            "Frost Data must be a static expression, got \"{}\" at top-level",
            ast->node_label())};

    for (const auto* node : expr->walk())
    {
        if (not node->data_safe())
            throw Frost_Recoverable_Error{fmt::format(
                "Invalid node in Frost Data: {}", node->node_label())};
    }

    return expr->evaluate({});
}

void inject_meta(Symbol_Table& table)
{
    INJECT(read_data, 1, 1);
}
} // namespace frst
