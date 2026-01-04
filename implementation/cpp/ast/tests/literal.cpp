#include <catch2/catch_test_macros.hpp>

#include <frost/ast.hpp>

using frst::Value;
using frst::ast::Literal;

using namespace frst::literals;
using namespace std::literals;

TEST_CASE("Literal")
{
    frst::Symbol_Table table;
    std::ostringstream os;

    SECTION("Null Literal")
    {
        auto node = Literal{Value::create()};
        CHECK(node.evaluate(table)->is<frst::Null>());
        node.debug_dump_ast(os);
        CHECK(os.str() == "Literal(null)\n");
    }

    SECTION("Int Literal")
    {
        auto node = Literal{Value::create(42_f)};
        CHECK(node.evaluate(table)->get<frst::Int>() == 42_f);
        node.debug_dump_ast(os);
        CHECK(os.str() == "Literal(42)\n");
    }

    SECTION("Float Literal")
    {
        auto node = Literal{Value::create(3.14)};
        CHECK(node.evaluate(table)->get<frst::Float>() == 3.14);
        node.debug_dump_ast(os);
        auto str = os.str();
        CHECK(str.starts_with("Literal(3.14"));
        CHECK(str.ends_with(")\n"));
    }

    SECTION("Bool Literal")
    {
        auto node = Literal{Value::create(true)};
        CHECK(node.evaluate(table)->get<frst::Bool>() == true);
        node.debug_dump_ast(os);
        CHECK(os.str() == "Literal(true)\n");
    }

    SECTION("String Literal")
    {
        auto node = Literal{Value::create("Hello World!"s)};
        CHECK(node.evaluate(table)->get<frst::String>() == "Hello World!");
        node.debug_dump_ast(os);
        CHECK(os.str() == "Literal(\"Hello World!\")\n");
    }
}
