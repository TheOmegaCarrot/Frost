#include <catch2/catch_test_macros.hpp>

#include <frost/testing/stringmaker-specializations.hpp>
#include <frost/value.hpp>

#include <lexy/action/parse.hpp>
#include <lexy/callback.hpp>
#include <lexy/input/string_input.hpp>

#include "../grammar.hpp"

namespace
{
struct Literal_Root
{
    static constexpr auto whitespace = frst::grammar::ws;
    static constexpr auto rule =
        lexy::dsl::p<frst::grammar::literal> + lexy::dsl::eof;
    static constexpr auto value = lexy::forward<frst::Value_Ptr>;
};
} // namespace

TEST_CASE("Parser Keyword Literals")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).
    auto parse = [](std::string_view input) {
        auto src = lexy::string_input(input);
        return lexy::parse<Literal_Root>(src, lexy::noop);
    };

    SECTION("true and false")
    {
        auto res_true = parse("true");
        REQUIRE(res_true);
        auto val_true = std::move(res_true).value();
        REQUIRE(val_true->is<frst::Bool>());
        CHECK(val_true->get<frst::Bool>().value() == true);

        auto res_false = parse("false");
        REQUIRE(res_false);
        auto val_false = std::move(res_false).value();
        REQUIRE(val_false->is<frst::Bool>());
        CHECK(val_false->get<frst::Bool>().value() == false);
    }

    SECTION("null")
    {
        auto res_null = parse("null");
        REQUIRE(res_null);
        auto val_null = std::move(res_null).value();
        REQUIRE(val_null->is<frst::Null>());
    }
}
