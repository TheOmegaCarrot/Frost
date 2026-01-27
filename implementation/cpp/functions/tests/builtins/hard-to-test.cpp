// AI-generated test by Codex (GPT-5).
// Signed: Codex (GPT-5).
#include <catch2/catch_all.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

using namespace frst;

TEST_CASE("Builtin hard-to-test injection")
{
    Symbol_Table table;
    inject_builtins(table);

    const std::vector<std::string> names{
        "print",
        "mprint",
        "getenv",
        "exit",
    };

    for (const auto& name : names)
    {
        auto val = table.lookup(name);
        REQUIRE(val->is<Function>());
    }
}
