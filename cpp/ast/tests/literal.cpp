#include <catch2/catch_test_macros.hpp>

#include <frost/ast.hpp>

#include <frost/mock/mock-symbol-table.hpp>

using namespace frst;
using ast::Literal;

using namespace literals;
using namespace std::literals;

TEST_CASE("Literal")
{
    mock::Mock_Symbol_Table table;
    std::ostringstream os;

    SECTION("Null Literal")
    {
        auto node = Literal{Value::null()};
        CHECK(node.evaluate(table)->is<Null>());
        node.debug_dump_ast(os);
        CHECK(os.str() == "Literal(null)\n");
    }

    SECTION("Int Literal")
    {
        auto node = Literal{Value::create(42_f)};
        CHECK(node.evaluate(table)->get<Int>() == 42_f);
        node.debug_dump_ast(os);
        CHECK(os.str() == "Literal(42)\n");
    }

    SECTION("Float Literal")
    {
        auto node = Literal{Value::create(3.14)};
        CHECK(node.evaluate(table)->get<Float>() == 3.14);
        node.debug_dump_ast(os);
        auto str = os.str();
        CHECK(str.starts_with("Literal(3.14"));
        CHECK(str.ends_with(")\n"));
    }

    SECTION("Bool Literal")
    {
        auto node = Literal{Value::create(true)};
        CHECK(node.evaluate(table)->get<Bool>() == true);
        node.debug_dump_ast(os);
        CHECK(os.str() == "Literal(true)\n");
    }

    SECTION("String Literal")
    {
        auto node = Literal{Value::create("Hello World!"s)};
        CHECK(node.evaluate(table)->get<String>() == "Hello World!");
        node.debug_dump_ast(os);
        CHECK(os.str() == "Literal(\"Hello World!\")\n");
    }
}
