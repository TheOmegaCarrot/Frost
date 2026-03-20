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
    Evaluation_Context ctx{.symbols = table};
    std::ostringstream os;

    SECTION("Null Literal")
    {
        auto node = Literal{ast::Statement::no_range, Value::null()};
        CHECK(node.evaluate(ctx)->is<Null>());
        node.debug_dump_ast(os);
        CHECK(os.str() == "Literal(null) [0:0-0:0]\n");
    }

    SECTION("Int Literal")
    {
        auto node = Literal{ast::Statement::no_range, Value::create(42_f)};
        CHECK(node.evaluate(ctx)->get<Int>() == 42_f);
        node.debug_dump_ast(os);
        CHECK(os.str() == "Literal(42) [0:0-0:0]\n");
    }

    SECTION("Float Literal")
    {
        auto node = Literal{ast::Statement::no_range, Value::create(3.14)};
        CHECK(node.evaluate(ctx)->get<Float>() == 3.14);
        node.debug_dump_ast(os);
        auto str = os.str();
        CHECK(str.starts_with("Literal(3.14"));
        CHECK(str.ends_with("[0:0-0:0]\n"));
    }

    SECTION("Bool Literal")
    {
        auto node = Literal{ast::Statement::no_range, Value::create(true)};
        CHECK(node.evaluate(ctx)->get<Bool>() == true);
        node.debug_dump_ast(os);
        CHECK(os.str() == "Literal(true) [0:0-0:0]\n");
    }

    SECTION("String Literal")
    {
        auto node =
            Literal{ast::Statement::no_range, Value::create("Hello World!"s)};
        CHECK(node.evaluate(ctx)->get<String>() == "Hello World!");
        node.debug_dump_ast(os);
        CHECK(os.str() == "Literal(\"Hello World!\") [0:0-0:0]\n");
    }
}
