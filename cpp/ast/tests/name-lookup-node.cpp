#include <catch2/catch_test_macros.hpp>

#include <frost/ast.hpp>

#include <frost/mock/mock-symbol-table.hpp>

using namespace frst;

using namespace literals;
using namespace std::literals;

TEST_CASE("Name Lookup")
{
    mock::Mock_Symbol_Table syms;
    Evaluation_Context ctx{.symbols = syms};
    std::string name = "foo";
    auto value = Value::create(42_f);

    SECTION("Found")
    {
        REQUIRE_CALL(syms, lookup(name)).RETURN(value);

        ast::Name_Lookup node{ast::Statement::no_range, name};

        auto res = node.evaluate(ctx);
        CHECK(res == value);
    }

    SECTION("Not found")
    {
        REQUIRE_CALL(syms, lookup(name))
            .THROW(Frost_Recoverable_Error{"Uh oh"});

        ast::Name_Lookup node{ast::Statement::no_range, name};

        CHECK_THROWS(node.evaluate(ctx));
    }

    SECTION("Reject _")
    {
        CHECK_THROWS((ast::Name_Lookup{ast::Statement::no_range, "_"}));
    }
}
