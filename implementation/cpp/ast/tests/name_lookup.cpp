#include <catch2/catch_test_macros.hpp>

#include <frost/ast.hpp>

#include <frost/mock/mock-symbol-table.hpp>

using namespace frst;

using namespace literals;
using namespace std::literals;

TEST_CASE("Name Lookup")
{
    mock::Mock_Symbol_Table syms;
    std::string name = "foo";
    auto value = Value::create(42_f);

    SECTION("Found")
    {
        REQUIRE_CALL(syms, lookup(name)).RETURN(value);

        ast::Name_Lookup node{name};

        auto res = node.evaluate(syms);
        CHECK(res == value);
    }

    SECTION("Not found")
    {
        REQUIRE_CALL(syms, lookup(name)).THROW(Frost_User_Error{"Uh oh"});

        ast::Name_Lookup node{name};

        CHECK_THROWS(node.evaluate(syms));
    }
}
