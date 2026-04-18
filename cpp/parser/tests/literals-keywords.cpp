#include <catch2/catch_test_macros.hpp>

#include <frost/ast.hpp>
#include <frost/parser.hpp>
#include <frost/symbol-table.hpp>
#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

TEST_CASE("Parser Keyword Literals")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        return frst::parse_data(std::string{input});
    };

    SECTION("true and false")
    {
        auto res_true = parse("true");
        REQUIRE(res_true.has_value());
        auto expr_true = std::move(res_true).value();
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto val_true = expr_true->evaluate(ctx);
        REQUIRE(val_true->is<frst::Bool>());
        CHECK(val_true->get<frst::Bool>().value() == true);

        auto res_false = parse("false");
        REQUIRE(res_false.has_value());
        auto expr_false = std::move(res_false).value();
        auto val_false = expr_false->evaluate(ctx);
        REQUIRE(val_false->is<frst::Bool>());
        CHECK(val_false->get<frst::Bool>().value() == false);
    }

    SECTION("null")
    {
        auto res_null = parse("null");
        REQUIRE(res_null.has_value());
        auto expr_null = std::move(res_null).value();
        frst::Symbol_Table table;
        frst::Evaluation_Context ctx{.symbols = table};
        auto val_null = expr_null->evaluate(ctx);
        REQUIRE(val_null->is<frst::Null>());
    }
}
