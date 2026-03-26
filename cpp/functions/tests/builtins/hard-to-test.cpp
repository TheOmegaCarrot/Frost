// AI-generated test by Codex (GPT-5).
// Signed: Codex (GPT-5).
#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

using namespace frst;
using Catch::Matchers::ContainsSubstring;

TEST_CASE("Builtin hard-to-test injection")
{
    Symbol_Table table;
    inject_builtins(table);

    const std::vector<std::string> names{
        "print",
        "mprint",
    };

    for (const auto& name : names)
    {
        auto val = table.lookup(name);
        REQUIRE(val->is<Function>());
    }
}

TEST_CASE("Builtin print arity")
{
    Symbol_Table table;
    inject_builtins(table);

    auto print_val = table.lookup("print");
    REQUIRE(print_val->is<Function>());
    auto print_fn = print_val->get<Function>().value();

    CHECK_THROWS_WITH(print_fn->call({}),
                      ContainsSubstring("insufficient arguments"));
    CHECK_THROWS_WITH(
        print_fn->call({Value::null(), Value::null()}),
        ContainsSubstring("too many arguments"));
}
