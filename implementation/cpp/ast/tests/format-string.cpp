#include <catch2/catch_test_macros.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <catch2/trompeloeil.hpp>

#include <frost/mock/mock-symbol-table.hpp>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/ast.hpp>
#include <frost/symbol-table.hpp>

#include <ranges>
#include <vector>

using namespace frst;
using namespace frst::ast;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::MessageMatches;

using trompeloeil::_;

namespace
{
std::string action_to_string(const Statement::Symbol_Action& action)
{
    return action.visit(Overload{
        [](const Statement::Definition& action) {
            return "def:" + action.name;
        },
        [](const Statement::Usage& action) {
            return "use:" + action.name;
        },
    });
}

std::vector<std::string> collect_sequence(const Statement& node)
{
    return node.symbol_sequence()
           | std::views::transform(&action_to_string)
           | std::ranges::to<std::vector>();
}
} // namespace

TEST_CASE("Format_String")
{
    // AI-generated test by Codex (GPT-5).
    // Signed: Codex (GPT-5).

    SECTION("Literal-only string performs no lookups")
    {
        mock::Mock_Symbol_Table syms;
        Format_String node{"hello world"};

        FORBID_CALL(syms, lookup(_));

        auto result = node.evaluate(syms);
        REQUIRE(result->is<frst::String>());
        CHECK(result->get<frst::String>() == "hello world");
    }

    SECTION("Single placeholder is substituted using to_internal_string")
    {
        mock::Mock_Symbol_Table syms;
        auto value = Value::create(42_f);

        REQUIRE_CALL(syms, lookup("n")).RETURN(value);

        Format_String node{"value: ${n}"};
        auto result = node.evaluate(syms);
        REQUIRE(result->is<frst::String>());
        CHECK(result->get<frst::String>() == "value: 42");
    }

    SECTION("Multiple placeholders are evaluated in order")
    {
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);

        REQUIRE_CALL(syms, lookup("a")).IN_SEQUENCE(seq).RETURN(a);
        REQUIRE_CALL(syms, lookup("b")).IN_SEQUENCE(seq).RETURN(b);

        Format_String node{"${a} + ${b}"};
        auto result = node.evaluate(syms);
        REQUIRE(result->is<frst::String>());
        CHECK(result->get<frst::String>() == "1 + 2");
    }

    SECTION("Escaped placeholders are treated as literal text")
    {
        mock::Mock_Symbol_Table syms;
        auto value = Value::create(1_f);

        REQUIRE_CALL(syms, lookup("name")).RETURN(value);

        Format_String node{R"(\\${name} \${name})"};
        auto result = node.evaluate(syms);
        REQUIRE(result->is<frst::String>());
        CHECK(result->get<frst::String>() == R"(\1 ${name})");
    }

    SECTION("Literal dollar signs remain literal")
    {
        mock::Mock_Symbol_Table syms;
        Format_String node{"price $5"};

        FORBID_CALL(syms, lookup(_));

        auto result = node.evaluate(syms);
        REQUIRE(result->is<frst::String>());
        CHECK(result->get<frst::String>() == "price $5");
    }

    SECTION("$${name} yields a literal $ then a placeholder")
    {
        mock::Mock_Symbol_Table syms;
        auto value = Value::create(3_f);

        REQUIRE_CALL(syms, lookup("name")).RETURN(value);

        Format_String node{"$${name}"};
        auto result = node.evaluate(syms);
        REQUIRE(result->is<frst::String>());
        CHECK(result->get<frst::String>() == "$3");
    }

    SECTION("Adjacent placeholders concatenate")
    {
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto a = Value::create(1_f);
        auto b = Value::create(2_f);

        REQUIRE_CALL(syms, lookup("a")).IN_SEQUENCE(seq).RETURN(a);
        REQUIRE_CALL(syms, lookup("b")).IN_SEQUENCE(seq).RETURN(b);

        Format_String node{"${a}${b}"};
        auto result = node.evaluate(syms);
        REQUIRE(result->is<frst::String>());
        CHECK(result->get<frst::String>() == "12");
    }

    SECTION("Repeated placeholder name is looked up each time")
    {
        mock::Mock_Symbol_Table syms;
        trompeloeil::sequence seq;

        auto value = Value::create(7_f);

        REQUIRE_CALL(syms, lookup("x")).IN_SEQUENCE(seq).RETURN(value);
        REQUIRE_CALL(syms, lookup("x")).IN_SEQUENCE(seq).RETURN(value);

        Format_String node{"${x}${x}"};
        auto result = node.evaluate(syms);
        REQUIRE(result->is<frst::String>());
        CHECK(result->get<frst::String>() == "77");
    }

    SECTION("Backslashes without placeholders are preserved")
    {
        mock::Mock_Symbol_Table syms;
        Format_String node{R"(\\)"};

        FORBID_CALL(syms, lookup(_));

        auto result = node.evaluate(syms);
        REQUIRE(result->is<frst::String>());
        CHECK(result->get<frst::String>() == R"(\)");
    }

    SECTION("Null values format as null")
    {
        mock::Mock_Symbol_Table syms;

        REQUIRE_CALL(syms, lookup("x")).RETURN(Value::null());

        Format_String node{"value: ${x}"};
        auto result = node.evaluate(syms);
        REQUIRE(result->is<frst::String>());
        CHECK(result->get<frst::String>() == "value: null");
    }

    SECTION("Missing name lookup raises an unrecoverable error")
    {
        Symbol_Table syms;
        Format_String node{"${missing}"};

        CHECK_THROWS_MATCHES(node.evaluate(syms), Frost_Unrecoverable_Error,
                             MessageMatches(ContainsSubstring("missing")));
    }

    SECTION("Constructor rejects malformed placeholders")
    {
        CHECK_THROWS(Format_String("${}"));
        CHECK_THROWS(Format_String("${1}"));
        CHECK_THROWS(Format_String("${foo${bar}}"));
    }

    SECTION("symbol_sequence yields placeholder usages in order")
    {
        Format_String node{R"(a ${x} b \${y} ${z})"};
        auto actions = collect_sequence(node);
        REQUIRE(actions.size() == 2);
        CHECK(actions[0] == "use:x");
        CHECK(actions[1] == "use:z");
    }

    SECTION("symbol_sequence includes repeated placeholders in order")
    {
        Format_String node{"${x}${x}"};
        auto actions = collect_sequence(node);
        REQUIRE(actions.size() == 2);
        CHECK(actions[0] == "use:x");
        CHECK(actions[1] == "use:x");
    }
}
