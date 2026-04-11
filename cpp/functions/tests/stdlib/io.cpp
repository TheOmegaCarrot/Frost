#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_exception.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <cctype>
#include <filesystem>
#include <string>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/stdlib.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using Catch::Matchers::ContainsSubstring;
using Catch::Matchers::MessageMatches;

namespace
{

Map io_module()
{
    Stdlib_Registry_Builder builder;
    register_module_io(builder);
    auto registry = std::move(builder).build();
    auto module = registry.lookup_module("std.io");
    REQUIRE(module.has_value());
    REQUIRE(module.value()->is<Map>());
    return module.value()->raw_get<Map>();
}

Function lookup(const Map& mod, const std::string& name)
{
    auto key = Value::create(String{name});
    auto it = mod.find(key);
    REQUIRE(it != mod.end());
    REQUIRE(it->second->is<Function>());
    return it->second->raw_get<Function>();
}

Function get_map_fn(const Value_Ptr& map_val, std::string_view name)
{
    REQUIRE(map_val->is<Map>());
    const auto& map = map_val->raw_get<Map>();
    auto key = Value::create(std::string{name});
    for (const auto& [k, v] : map)
    {
        if (Value::equal(k, key)->get<Bool>().value())
        {
            REQUIRE(v);
            REQUIRE(v->is<Function>());
            return v->get<Function>().value();
        }
    }
    FAIL("Missing map entry");
    return Function{};
}

std::filesystem::path make_test_dir(std::string_view test_name)
{
    std::string safe;
    safe.reserve(test_name.size());
    for (char c : test_name)
    {
        if (std::isalnum(static_cast<unsigned char>(c)))
            safe.push_back(c);
        else
            safe.push_back('_');
    }

    auto dir = std::filesystem::path{"./build/streams"} / safe;
    std::filesystem::create_directories(dir);
    return dir;
}

std::filesystem::path unique_path(std::filesystem::path base,
                                  std::string_view name)
{
    static std::size_t counter = 0;
    return base
           / (std::string{name} + "_" + std::to_string(counter++) + ".txt");
}

} // namespace

TEST_CASE("std.io")
{
    auto mod = io_module();

    SECTION("Registered")
    {
        CHECK(mod.size() == 5);
        lookup(mod, "open_read");
        lookup(mod, "open_trunc");
        lookup(mod, "open_append");
        lookup(mod, "stringreader");
        lookup(mod, "stringwriter");
    }
}

TEST_CASE("std.io stringreader")
{
    auto mod = io_module();
    auto reader_fn = lookup(mod, "stringreader");

    SECTION("Arity and type errors")
    {
        CHECK_THROWS_MATCHES(
            reader_fn->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 1")));
        CHECK_THROWS_MATCHES(reader_fn->call({Value::create(1_f)}),
                             Frost_User_Error,
                             MessageMatches(ContainsSubstring("io.stringreader")
                                            && ContainsSubstring("String")
                                            && ContainsSubstring("Int")));
    }

    SECTION("Read behaviors")
    {
        auto reader_map = reader_fn->call({Value::create("a\nb"s)});
        auto read_line = get_map_fn(reader_map, "read_line");
        auto read_one = get_map_fn(reader_map, "read_one");
        auto read_rest = get_map_fn(reader_map, "read_rest");
        auto eof = get_map_fn(reader_map, "eof");
        auto tell = get_map_fn(reader_map, "tell");
        auto seek = get_map_fn(reader_map, "seek");

        auto first_line = read_line->call({});
        REQUIRE(first_line->is<String>());
        CHECK(first_line->get<String>() == "a");

        auto second_line = read_line->call({});
        REQUIRE(second_line->is<String>());
        CHECK(second_line->get<String>() == "b");

        auto reader_map2 = reader_fn->call({Value::create("xy"s)});
        auto read_one2 = get_map_fn(reader_map2, "read_one");
        auto read_rest2 = get_map_fn(reader_map2, "read_rest");
        auto tell2 = get_map_fn(reader_map2, "tell");
        auto seek2 = get_map_fn(reader_map2, "seek");

        auto first_char = read_one2->call({});
        REQUIRE(first_char->is<String>());
        CHECK(first_char->get<String>() == "x");
        auto pos = tell2->call({});
        REQUIRE(pos->is<Int>());
        CHECK(pos->get<Int>().value() == 1_f);

        seek2->call({Value::create(0_f)});
        auto rest = read_rest2->call({});
        REQUIRE(rest->is<String>());
        CHECK(rest->get<String>() == "xy");

        auto at_eof = eof->call({});
        REQUIRE(at_eof->is<Bool>());
    }

    SECTION("Seek type error")
    {
        auto reader_map = reader_fn->call({Value::create("abc"s)});
        auto seek = get_map_fn(reader_map, "seek");
        CHECK_THROWS_MATCHES(
            seek->call({Value::create("not_an_int"s)}), Frost_User_Error,
            MessageMatches(ContainsSubstring("<system closure:seek>")
                           && ContainsSubstring("pos")
                           && ContainsSubstring("String")));
    }

    SECTION("Read one returns null at EOF")
    {
        auto reader_map = reader_fn->call({Value::create(""s)});
        auto read_one = get_map_fn(reader_map, "read_one");
        auto got = read_one->call({});
        CHECK(got->is<Null>());
    }
}

TEST_CASE("std.io stringwriter")
{
    auto mod = io_module();
    auto writer_fn = lookup(mod, "stringwriter");

    SECTION("Arity error")
    {
        CHECK_THROWS_MATCHES(
            writer_fn->call({Value::create(1_f)}), Frost_User_Error,
            MessageMatches(ContainsSubstring("too many arguments")
                           && ContainsSubstring("no more than 0")));
    }

    SECTION("Write and writeln type errors")
    {
        auto writer_map = writer_fn->call({});
        auto write = get_map_fn(writer_map, "write");
        auto writeln = get_map_fn(writer_map, "writeln");
        CHECK_THROWS_MATCHES(
            write->call({Value::create(1_f)}), Frost_User_Error,
            MessageMatches(ContainsSubstring("<system closure:write>")
                           && ContainsSubstring("String")));
        CHECK_THROWS_MATCHES(
            writeln->call({Value::create(1_f)}), Frost_User_Error,
            MessageMatches(ContainsSubstring("<system closure:writeln>")
                           && ContainsSubstring("String")));
    }

    SECTION("Write and get")
    {
        auto writer_map = writer_fn->call({});
        auto write = get_map_fn(writer_map, "write");
        auto writeln = get_map_fn(writer_map, "writeln");
        auto get = get_map_fn(writer_map, "get");

        write->call({Value::create("hello"s)});
        writeln->call({Value::create("world"s)});

        auto result = get->call({});
        REQUIRE(result->is<String>());
        CHECK(result->get<String>() == "helloworld\n");
    }
}

TEST_CASE("std.io open_trunc")
{
    auto mod = io_module();
    auto open_trunc_fn = lookup(mod, "open_trunc");
    auto open_read_fn = lookup(mod, "open_read");

    SECTION("Arity and type errors")
    {
        CHECK_THROWS_MATCHES(
            open_trunc_fn->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 1")));
        CHECK_THROWS_MATCHES(open_trunc_fn->call({Value::create(1_f)}),
                             Frost_User_Error,
                             MessageMatches(ContainsSubstring("io.open_trunc")
                                            && ContainsSubstring("String")
                                            && ContainsSubstring("Int")
                                            && ContainsSubstring("path")));
    }

    SECTION("Open failure throws an error identifying the path")
    {
        // Opening a directory for writing must fail. The error should name
        // the path so the caller knows which target failed.
        auto dir = make_test_dir("std_io_open_trunc_fail");
        CHECK_THROWS_MATCHES(
            open_trunc_fn->call({Value::create(dir.string())}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("Failed to open")
                           && ContainsSubstring(dir.string())));
    }

    SECTION("Write then read")
    {
        auto dir = make_test_dir("std_io_open_trunc");
        auto path = unique_path(dir, "trunc");

        auto writer_map = open_trunc_fn->call({Value::create(path.string())});
        REQUIRE(writer_map->is<Map>());

        auto write = get_map_fn(writer_map, "write");
        auto tell = get_map_fn(writer_map, "tell");
        auto seek = get_map_fn(writer_map, "seek");
        auto close = get_map_fn(writer_map, "close");
        auto is_open = get_map_fn(writer_map, "is_open");

        auto open_val = is_open->call({});
        REQUIRE(open_val->is<Bool>());
        CHECK(open_val->get<Bool>().value() == true);

        auto start_pos = tell->call({});
        REQUIRE(start_pos->is<Int>());
        CHECK(start_pos->get<Int>().value() == 0_f);

        write->call({Value::create("abc"s)});
        auto after_write = tell->call({});
        REQUIRE(after_write->is<Int>());
        CHECK(after_write->get<Int>().value() == 3_f);

        seek->call({Value::create(1_f)});
        write->call({Value::create("Z"s)});
        close->call({});
        auto closed_val = is_open->call({});
        REQUIRE(closed_val->is<Bool>());
        CHECK(closed_val->get<Bool>().value() == false);

        auto reader_map = open_read_fn->call({Value::create(path.string())});
        auto read_rest = get_map_fn(reader_map, "read_rest");
        auto read_value = read_rest->call({});
        REQUIRE(read_value->is<String>());
        CHECK(read_value->get<String>() == "aZc");
    }

    SECTION("Truncates existing content")
    {
        // Re-opening an existing file with open_trunc must discard its prior
        // contents. If truncation were skipped, the new short write would leave
        // the tail of the old long write visible.
        auto dir = make_test_dir("std_io_open_trunc_truncates");
        auto path = unique_path(dir, "trunc");

        // Seed the file with long content.
        auto first = open_trunc_fn->call({Value::create(path.string())});
        auto first_write = get_map_fn(first, "write");
        auto first_close = get_map_fn(first, "close");
        first_write->call(
            {Value::create("original long content that should be gone"s)});
        first_close->call({});

        // Re-open the same path with open_trunc and write something shorter.
        auto second = open_trunc_fn->call({Value::create(path.string())});
        auto second_write = get_map_fn(second, "write");
        auto second_close = get_map_fn(second, "close");
        second_write->call({Value::create("new"s)});
        second_close->call({});

        // The result must be exactly "new" -- no trailing bytes from the
        // original content.
        auto reader = open_read_fn->call({Value::create(path.string())});
        auto read_rest = get_map_fn(reader, "read_rest");
        auto read_value = read_rest->call({});
        REQUIRE(read_value->is<String>());
        CHECK(read_value->get<String>() == "new");
    }
}

TEST_CASE("std.io open_append")
{
    auto mod = io_module();
    auto open_trunc_fn = lookup(mod, "open_trunc");
    auto open_append_fn = lookup(mod, "open_append");
    auto open_read_fn = lookup(mod, "open_read");

    SECTION("Arity and type errors")
    {
        CHECK_THROWS_MATCHES(
            open_append_fn->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 1")));
        CHECK_THROWS_MATCHES(open_append_fn->call({Value::create(1_f)}),
                             Frost_User_Error,
                             MessageMatches(ContainsSubstring("io.open_append")
                                            && ContainsSubstring("String")
                                            && ContainsSubstring("Int")
                                            && ContainsSubstring("path")));
    }

    SECTION("Open failure throws an error identifying the path")
    {
        // Opening a directory for writing must fail. The error should name
        // the path so the caller knows which target failed.
        auto dir = make_test_dir("std_io_open_append_fail");
        CHECK_THROWS_MATCHES(
            open_append_fn->call({Value::create(dir.string())}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("Failed to open")
                           && ContainsSubstring(dir.string())));
    }

    SECTION("Append writes after existing content")
    {
        auto dir = make_test_dir("std_io_open_append");
        auto path = unique_path(dir, "append");

        auto writer_map = open_trunc_fn->call({Value::create(path.string())});
        auto write = get_map_fn(writer_map, "write");
        auto close = get_map_fn(writer_map, "close");

        write->call({Value::create("a"s)});
        close->call({});

        auto append_map = open_append_fn->call({Value::create(path.string())});
        auto write_append = get_map_fn(append_map, "write");
        auto close_append = get_map_fn(append_map, "close");

        write_append->call({Value::create("b"s)});
        close_append->call({});

        auto reader_map = open_read_fn->call({Value::create(path.string())});
        auto read_rest = get_map_fn(reader_map, "read_rest");
        auto result = read_rest->call({});
        REQUIRE(result->is<String>());
        CHECK(result->get<String>() == "ab");
    }
}

TEST_CASE("std.io open_read")
{
    auto mod = io_module();
    auto open_trunc_fn = lookup(mod, "open_trunc");
    auto open_read_fn = lookup(mod, "open_read");

    SECTION("Arity and type errors")
    {
        CHECK_THROWS_MATCHES(
            open_read_fn->call({}), Frost_User_Error,
            MessageMatches(ContainsSubstring("insufficient arguments")
                           && ContainsSubstring("requires at least 1")));
        CHECK_THROWS_MATCHES(open_read_fn->call({Value::create(1_f)}),
                             Frost_User_Error,
                             MessageMatches(ContainsSubstring("io.open_read")
                                            && ContainsSubstring("String")
                                            && ContainsSubstring("Int")
                                            && ContainsSubstring("path")));
    }

    SECTION("Open failure throws an error identifying the path")
    {
        // Reading a nonexistent path must fail. The error should name
        // the path so the caller knows which target failed.
        const std::string missing =
            "./build/streams/definitely_nonexistent.txt";
        std::filesystem::remove(missing);
        CHECK_THROWS_MATCHES(
            open_read_fn->call({Value::create(String{missing})}),
            Frost_User_Error,
            MessageMatches(ContainsSubstring("Failed to open")
                           && ContainsSubstring(missing)));
    }

    SECTION("Read and eof")
    {
        auto dir = make_test_dir("std_io_open_read");
        auto path = unique_path(dir, "read");

        auto writer_map = open_trunc_fn->call({Value::create(path.string())});
        auto write = get_map_fn(writer_map, "write");
        auto close_writer = get_map_fn(writer_map, "close");

        write->call({Value::create("line1\nline2"s)});
        close_writer->call({});

        auto reader_map = open_read_fn->call({Value::create(path.string())});
        auto read_line = get_map_fn(reader_map, "read_line");
        auto read_rest = get_map_fn(reader_map, "read_rest");
        auto eof = get_map_fn(reader_map, "eof");
        auto tell = get_map_fn(reader_map, "tell");
        auto seek = get_map_fn(reader_map, "seek");
        auto close_reader = get_map_fn(reader_map, "close");
        auto is_open = get_map_fn(reader_map, "is_open");

        auto open_val = is_open->call({});
        REQUIRE(open_val->is<Bool>());
        CHECK(open_val->get<Bool>().value() == true);

        auto start_pos = tell->call({});
        REQUIRE(start_pos->is<Int>());
        CHECK(start_pos->get<Int>().value() == 0_f);

        auto first = read_line->call({});
        REQUIRE(first->is<String>());
        CHECK(first->get<String>() == "line1");

        auto rest = read_rest->call({});
        REQUIRE(rest->is<String>());
        CHECK(rest->get<String>() == "line2");

        auto at_eof = eof->call({});
        REQUIRE(at_eof->is<Bool>());

        seek->call({Value::create(0_f)});
        auto rewind_pos = tell->call({});
        REQUIRE(rewind_pos->is<Int>());
        CHECK(rewind_pos->get<Int>().value() == 0_f);

        close_reader->call({});
        auto closed_val = is_open->call({});
        REQUIRE(closed_val->is<Bool>());
        CHECK(closed_val->get<Bool>().value() == false);
    }
}
