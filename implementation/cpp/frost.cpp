#include <frost/ast.hpp>
#include <frost/builtin.hpp>
#include <frost/parser.hpp>
#include <frost/symbol-table.hpp>

#include <filesystem>

#include <fmt/format.h>

int main(int argc, char** argv)
{

    if (argc != 2)
        return 1;

    auto result = frst::parse_file(argv[1]);

    if (!result)
        fmt::println(stderr, "{}", result.error());

    frst::Symbol_Table symbols;
    frst::inject_builtins(symbols);

    for (const auto& statement : result.value())
    {
        statement->execute(symbols);
    }
}
