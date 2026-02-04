// AI-generated test by Codex (GPT-5).
// Signed: Codex (GPT-5).
#include <catch2/catch_all.hpp>
#include <catch2/matchers/catch_matchers_string.hpp>

#include <boost/scope_exit.hpp>

#include <atomic>
#include <chrono>
#include <filesystem>
#include <fstream>
#include <random>
#include <string>
#include <vector>

#include <frost/testing/stringmaker-specializations.hpp>

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>
#include <frost/value.hpp>

using namespace frst;
using namespace std::literals;
using namespace Catch::Matchers;

namespace
{
Function get_fs_fn(const Value_Ptr& fs_val, std::string_view name)
{
    REQUIRE(fs_val->is<Map>());
    const auto& fs_map = fs_val->raw_get<Map>();
    auto key = Value::create(std::string{name});
    for (const auto& [k, v] : fs_map)
    {
        if (Value::equal(k, key)->get<Bool>().value())
        {
            REQUIRE(v);
            REQUIRE(v->is<Function>());
            return v->get<Function>().value();
        }
    }
    FAIL("Missing fs function");
    return Function{};
}

std::filesystem::path make_temp_dir(std::string_view test_name)
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

    static std::atomic<std::uint64_t> counter{0};
    const auto stamp =
        std::chrono::steady_clock::now().time_since_epoch().count();
    const auto id = counter.fetch_add(1, std::memory_order_relaxed);
    static thread_local std::mt19937_64 rng{std::random_device{}()};
    const auto nonce = rng();

    auto dir = std::filesystem::current_path()
               / "tmp"
               / "frost_fs_tests"
               / (safe
                  + "_"
                  + std::to_string(stamp)
                  + "_"
                  + std::to_string(id)
                  + "_"
                  + std::to_string(nonce));
    std::filesystem::create_directories(dir);
    return dir;
}

std::filesystem::path path_from_value(const Value_Ptr& val)
{
    REQUIRE(val->is<String>());
    return std::filesystem::path{val->raw_get<String>()};
}

bool contains_path(const Array& arr, const std::filesystem::path& expected)
{
    auto expected_norm = expected.lexically_normal();
    for (const auto& entry : arr)
    {
        auto got = path_from_value(entry).lexically_normal();
        if (got == expected_norm)
            return true;
    }
    return false;
}
} // namespace

TEST_CASE("Builtin filesystem injection")
{
    Symbol_Table table;
    inject_builtins(table);

    auto fs_val = table.lookup("fs");
    REQUIRE(fs_val->is<Map>());
    const auto& fs_map = fs_val->raw_get<Map>();

    const std::vector<std::string> names{
        "move",   "symlink", "copy",   "absolute", "canonical",
        "cd",     "cwd",     "exists", "remove",   "remove_recursively",
        "mkdir",  "size",    "stat",   "list",     "list_recursively",
        "concat",
    };

    SECTION("Injected")
    {
        REQUIRE(fs_map.size() == names.size());
        for (const auto& name : names)
        {
            auto fn = get_fs_fn(fs_val, name);
            REQUIRE(fn);
        }
    }

    SECTION("Arity")
    {
        struct Arity_Case
        {
            std::string name;
            std::size_t min;
            std::size_t max;
        };

        const std::vector<Arity_Case> cases{
            {"move", 2, 2},
            {"symlink", 2, 2},
            {"copy", 2, 2},
            {"absolute", 1, 1},
            {"canonical", 1, 1},
            {"cd", 1, 1},
            {"cwd", 0, 0},
            {"exists", 1, 1},
            {"remove", 1, 1},
            {"remove_recursively", 1, 1},
            {"mkdir", 1, 1},
            {"size", 1, 1},
            {"stat", 1, 1},
            {"list", 1, 1},
            {"list_recursively", 1, 1},
            {"concat", 2, 2},
        };

        auto a = Value::create("a"s);
        auto b = Value::create("b"s);
        auto c = Value::create("c"s);

        for (const auto& test : cases)
        {
            DYNAMIC_SECTION("Arity " << test.name)
            {
                auto fn = get_fs_fn(fs_val, test.name);

                if (test.min == 0)
                {
                    CHECK_THROWS_MATCHES(
                        fn->call({a}), Frost_User_Error,
                        MessageMatches(ContainsSubstring("too many arguments")
                                       && ContainsSubstring("Called with 1")
                                       && ContainsSubstring("no more than 0")));
                }
                else if (test.min == 1)
                {
                    CHECK_THROWS_MATCHES(
                        fn->call({}), Frost_User_Error,
                        MessageMatches(
                            ContainsSubstring("insufficient arguments")
                            && ContainsSubstring("Called with 0")
                            && ContainsSubstring("requires at least 1")));
                }
                else if (test.min == 2)
                {
                    CHECK_THROWS_MATCHES(
                        fn->call({a}), Frost_User_Error,
                        MessageMatches(
                            ContainsSubstring("insufficient arguments")
                            && ContainsSubstring("Called with 1")
                            && ContainsSubstring("requires at least 2")));
                }

                if (test.max == 1)
                {
                    CHECK_THROWS_MATCHES(
                        fn->call({a, b}), Frost_User_Error,
                        MessageMatches(ContainsSubstring("too many arguments")
                                       && ContainsSubstring("Called with 2")
                                       && ContainsSubstring("no more than 1")));
                }
                else if (test.max == 2)
                {
                    CHECK_THROWS_MATCHES(
                        fn->call({a, b, c}), Frost_User_Error,
                        MessageMatches(ContainsSubstring("too many arguments")
                                       && ContainsSubstring("Called with 3")
                                       && ContainsSubstring("no more than 2")));
                }
            }
        }
    }

    SECTION("Type errors")
    {
        struct Type_Case
        {
            std::string name;
            std::size_t arity;
        };

        const std::vector<Type_Case> cases{
            {"move", 2},
            {"symlink", 2},
            {"copy", 2},
            {"absolute", 1},
            {"canonical", 1},
            {"cd", 1},
            {"exists", 1},
            {"remove", 1},
            {"remove_recursively", 1},
            {"mkdir", 1},
            {"size", 1},
            {"stat", 1},
            {"list", 1},
            {"list_recursively", 1},
            {"concat", 2},
        };

        auto bad = Value::create(1_f);

        for (const auto& test : cases)
        {
            DYNAMIC_SECTION("Type " << test.name)
            {
                auto fn = get_fs_fn(fs_val, test.name);
                if (test.arity == 1)
                {
                    CHECK_THROWS_AS(fn->call({bad}), Frost_User_Error);
                }
                else
                {
                    CHECK_THROWS_AS(fn->call({bad, bad}), Frost_User_Error);
                }
            }
        }
    }
}

TEST_CASE("Builtin filesystem operations")
{
    Symbol_Table table;
    inject_builtins(table);

    auto fs_val = table.lookup("fs");
    REQUIRE(fs_val->is<Map>());

    SECTION("cwd and cd")
    {
        auto cwd_fn = get_fs_fn(fs_val, "cwd");
        auto cd_fn = get_fs_fn(fs_val, "cd");

        auto dir = make_temp_dir("fs_cwd");
        BOOST_SCOPE_EXIT_ALL(&dir)
        {
            std::error_code ec;
            std::filesystem::remove_all(dir, ec);
        };

        auto original = std::filesystem::current_path();
        BOOST_SCOPE_EXIT_ALL(&original)
        {
            std::error_code ec;
            std::filesystem::current_path(original, ec);
        };

        auto cwd_val = cwd_fn->call({});
        REQUIRE(cwd_val->is<String>());
        CHECK(std::filesystem::path{cwd_val->raw_get<String>()}
              == std::filesystem::current_path());

        auto cd_val = cd_fn->call({Value::create(dir.string())});
        REQUIRE(cd_val->is<Null>());
        auto new_cwd = cwd_fn->call({});
        REQUIRE(new_cwd->is<String>());
        CHECK(std::filesystem::path{new_cwd->raw_get<String>()} == dir);

        CHECK_THROWS_AS(
            cd_fn->call({Value::create((dir / "missing").string())}),
            Frost_Recoverable_Error);
    }

    SECTION("mkdir, exists, remove, remove_recursively")
    {
        auto mkdir_fn = get_fs_fn(fs_val, "mkdir");
        auto exists_fn = get_fs_fn(fs_val, "exists");
        auto remove_fn = get_fs_fn(fs_val, "remove");
        auto remove_all_fn = get_fs_fn(fs_val, "remove_recursively");

        auto dir = make_temp_dir("fs_dirs");
        BOOST_SCOPE_EXIT_ALL(&dir)
        {
            std::error_code ec;
            std::filesystem::remove_all(dir, ec);
        };

        auto nested = dir / "a" / "b";
        auto made = mkdir_fn->call({Value::create(nested.string())});
        REQUIRE(made->is<Bool>());
        CHECK(made->get<Bool>().value());

        auto made_again = mkdir_fn->call({Value::create(nested.string())});
        REQUIRE(made_again->is<Bool>());
        CHECK_FALSE(made_again->get<Bool>().value());

        auto exists_dir = exists_fn->call({Value::create(nested.string())});
        REQUIRE(exists_dir->is<Bool>());
        CHECK(exists_dir->get<Bool>().value());

        auto exists_missing =
            exists_fn->call({Value::create((dir / "missing").string())});
        REQUIRE(exists_missing->is<Bool>());
        CHECK_FALSE(exists_missing->get<Bool>().value());

        auto file = nested / "file.txt";
        {
            std::ofstream out(file);
            out << "hello";
        }

        auto exists_file = exists_fn->call({Value::create(file.string())});
        REQUIRE(exists_file->is<Bool>());
        CHECK(exists_file->get<Bool>().value());

        auto removed = remove_fn->call({Value::create(file.string())});
        REQUIRE(removed->is<Bool>());
        CHECK(removed->get<Bool>().value());

        auto removed_again = remove_fn->call({Value::create(file.string())});
        REQUIRE(removed_again->is<Bool>());
        CHECK_FALSE(removed_again->get<Bool>().value());

        auto tree = dir / "tree";
        std::filesystem::create_directories(tree / "sub");
        std::ofstream(tree / "a.txt") << "a";
        std::ofstream(tree / "sub" / "b.txt") << "b";

        auto removed_all = remove_all_fn->call({Value::create(tree.string())});
        REQUIRE(removed_all->is<Int>());
        CHECK(removed_all->get<Int>().value() > 0_f);
        CHECK_FALSE(std::filesystem::exists(tree));

        auto removed_missing = remove_all_fn->call(
            {Value::create((dir / "missing_tree").string())});
        REQUIRE(removed_missing->is<Int>());
        CHECK(removed_missing->get<Int>().value() == 0_f);
    }

    SECTION("size, absolute, canonical, concat")
    {
        auto size_fn = get_fs_fn(fs_val, "size");
        auto abs_fn = get_fs_fn(fs_val, "absolute");
        auto canon_fn = get_fs_fn(fs_val, "canonical");
        auto concat_fn = get_fs_fn(fs_val, "concat");

        auto dir = make_temp_dir("fs_paths");
        BOOST_SCOPE_EXIT_ALL(&dir)
        {
            std::error_code ec;
            std::filesystem::remove_all(dir, ec);
        };

        auto file = dir / "data.txt";
        {
            std::ofstream out(file);
            out << "hello";
        }

        auto size_val = size_fn->call({Value::create(file.string())});
        REQUIRE(size_val->is<Int>());
        CHECK(size_val->get<Int>().value() == 5_f);

        auto empty_file = dir / "empty.txt";
        std::ofstream{empty_file};
        auto empty_size = size_fn->call({Value::create(empty_file.string())});
        REQUIRE(empty_size->is<Int>());
        CHECK(empty_size->get<Int>().value() == 0_f);

        CHECK_THROWS_AS(
            size_fn->call({Value::create((dir / "missing").string())}),
            Frost_Recoverable_Error);

        CHECK_THROWS_AS(size_fn->call({Value::create(dir.string())}),
                        Frost_Recoverable_Error);

        auto abs_val = abs_fn->call({Value::create(file.string())});
        REQUIRE(abs_val->is<String>());
        CHECK(std::filesystem::path{abs_val->raw_get<String>()}
              == std::filesystem::absolute(file));

        auto canon_val = canon_fn->call({Value::create(file.string())});
        REQUIRE(canon_val->is<String>());
        CHECK(std::filesystem::path{canon_val->raw_get<String>()}
              == std::filesystem::canonical(file));

        CHECK_THROWS_AS(
            canon_fn->call({Value::create((dir / "missing").string())}),
            Frost_Recoverable_Error);

        auto concat_val = concat_fn->call(
            {Value::create(dir.string()), Value::create("child"s)});
        REQUIRE(concat_val->is<String>());
        CHECK(std::filesystem::path{concat_val->raw_get<String>()}
              == (dir / "child"));

        auto abs_rhs = std::filesystem::absolute(dir / "abs.txt");
        auto concat_abs = concat_fn->call(
            {Value::create(dir.string()), Value::create(abs_rhs.string())});
        REQUIRE(concat_abs->is<String>());
        CHECK(std::filesystem::path{concat_abs->raw_get<String>()} == abs_rhs);
    }

    SECTION("move and copy")
    {
        auto move_fn = get_fs_fn(fs_val, "move");
        auto copy_fn = get_fs_fn(fs_val, "copy");

        auto dir = make_temp_dir("fs_move_copy");
        BOOST_SCOPE_EXIT_ALL(&dir)
        {
            std::error_code ec;
            std::filesystem::remove_all(dir, ec);
        };

        auto src = dir / "src.txt";
        {
            std::ofstream out(src);
            out << "data";
        }
        auto dest = dir / "dest.txt";

        auto move_res = move_fn->call(
            {Value::create(src.string()), Value::create(dest.string())});
        REQUIRE(move_res->is<Null>());
        CHECK_FALSE(std::filesystem::exists(src));
        CHECK(std::filesystem::exists(dest));

        CHECK_THROWS_AS(
            move_fn->call({Value::create((dir / "missing").string()),
                           Value::create((dir / "missing_dest").string())}),
            Frost_Recoverable_Error);

        auto src_dir = dir / "src_dir";
        auto nested = src_dir / "nested";
        std::filesystem::create_directories(nested);
        std::ofstream(nested / "file.txt") << "nested";

        auto dest_dir = dir / "dest_dir";
        auto copy_res = copy_fn->call({Value::create(src_dir.string()),
                                       Value::create(dest_dir.string())});
        REQUIRE(copy_res->is<Null>());
        CHECK(std::filesystem::exists(dest_dir / "nested" / "file.txt"));

        CHECK_THROWS_AS(
            copy_fn->call({Value::create((dir / "missing_src").string()),
                           Value::create((dir / "missing_dest").string())}),
            Frost_Recoverable_Error);

        auto file_src = dir / "file_src.txt";
        auto file_dest_dir = dir / "dest_dir_exists";
        std::ofstream(file_src) << "file";
        std::filesystem::create_directories(file_dest_dir);
        CHECK_THROWS_AS(move_fn->call({Value::create(file_src.string()),
                                       Value::create(file_dest_dir.string())}),
                        Frost_Recoverable_Error);

        auto copy_src = dir / "copy_src.txt";
        auto copy_dest = dir / "copy_dest.txt";
        std::ofstream(copy_src) << "copy";
        std::ofstream(copy_dest) << "dest";
        CHECK_THROWS_AS(copy_fn->call({Value::create(copy_src.string()),
                                       Value::create(copy_dest.string())}),
                        Frost_Recoverable_Error);
    }

    SECTION("list and list_recursively")
    {
        auto list_fn = get_fs_fn(fs_val, "list");
        auto list_rec_fn = get_fs_fn(fs_val, "list_recursively");

        auto dir = make_temp_dir("fs_list");
        BOOST_SCOPE_EXIT_ALL(&dir)
        {
            std::error_code ec;
            std::filesystem::remove_all(dir, ec);
        };

        auto a = dir / "a.txt";
        auto b = dir / "b.txt";
        auto sub = dir / "sub";
        auto c = sub / "c.txt";
        std::filesystem::create_directories(sub);
        std::ofstream(a) << "a";
        std::ofstream(b) << "b";
        std::ofstream(c) << "c";

        auto list_val = list_fn->call({Value::create(dir.string())});
        REQUIRE(list_val->is<Array>());
        const auto& list_arr = list_val->raw_get<Array>();
        CHECK(contains_path(list_arr, a));
        CHECK(contains_path(list_arr, b));
        CHECK(contains_path(list_arr, sub));

        auto rec_val = list_rec_fn->call({Value::create(dir.string())});
        REQUIRE(rec_val->is<Array>());
        const auto& rec_arr = rec_val->raw_get<Array>();
        CHECK(contains_path(rec_arr, a));
        CHECK(contains_path(rec_arr, b));
        CHECK(contains_path(rec_arr, c));
        CHECK(contains_path(rec_arr, sub));

        auto empty = dir / "empty";
        std::filesystem::create_directories(empty);
        auto list_empty = list_fn->call({Value::create(empty.string())});
        REQUIRE(list_empty->is<Array>());
        CHECK(list_empty->raw_get<Array>().empty());
        auto rec_empty = list_rec_fn->call({Value::create(empty.string())});
        REQUIRE(rec_empty->is<Array>());
        CHECK(rec_empty->raw_get<Array>().empty());

        CHECK_THROWS_AS(list_fn->call({Value::create(a.string())}),
                        Frost_Recoverable_Error);
        CHECK_THROWS_AS(list_rec_fn->call({Value::create(a.string())}),
                        Frost_Recoverable_Error);

        CHECK_THROWS_AS(
            list_fn->call({Value::create((dir / "missing").string())}),
            Frost_Recoverable_Error);
        CHECK_THROWS_AS(
            list_rec_fn->call({Value::create((dir / "missing").string())}),
            Frost_Recoverable_Error);
    }

    SECTION("stat")
    {
        auto stat_fn = get_fs_fn(fs_val, "stat");

        auto dir = make_temp_dir("fs_stat");
        BOOST_SCOPE_EXIT_ALL(&dir)
        {
            std::error_code ec;
            std::filesystem::remove_all(dir, ec);
        };

        auto file = dir / "file.txt";
        std::ofstream(file) << "data";

        auto stat_file = stat_fn->call({Value::create(file.string())});
        REQUIRE(stat_file->is<Map>());
        auto type_key = Value::create("type"s);
        auto perms_key = Value::create("perms"s);

        const auto& file_map = stat_file->raw_get<Map>();
        auto type_it = file_map.find(type_key);
        REQUIRE(type_it != file_map.end());
        REQUIRE(type_it->second->is<String>());
        CHECK(type_it->second->get<String>() == "regular");

        auto perms_it = file_map.find(perms_key);
        REQUIRE(perms_it != file_map.end());
        REQUIRE(perms_it->second->is<Map>());
        const auto& perms_map = perms_it->second->raw_get<Map>();
        auto owner_it = perms_map.find(Value::create("owner"s));
        auto group_it = perms_map.find(Value::create("group"s));
        auto others_it = perms_map.find(Value::create("others"s));
        REQUIRE(owner_it != perms_map.end());
        REQUIRE(group_it != perms_map.end());
        REQUIRE(others_it != perms_map.end());
        REQUIRE(owner_it->second->is<Map>());
        REQUIRE(group_it->second->is<Map>());
        REQUIRE(others_it->second->is<Map>());

        auto check_perm_group = [](const Map& group) {
            auto read_it = group.find(Value::create("read"s));
            auto write_it = group.find(Value::create("write"s));
            auto exec_it = group.find(Value::create("exec"s));
            REQUIRE(read_it != group.end());
            REQUIRE(write_it != group.end());
            REQUIRE(exec_it != group.end());
            REQUIRE(read_it->second->is<Bool>());
            REQUIRE(write_it->second->is<Bool>());
            REQUIRE(exec_it->second->is<Bool>());
        };
        check_perm_group(owner_it->second->raw_get<Map>());
        check_perm_group(group_it->second->raw_get<Map>());
        check_perm_group(others_it->second->raw_get<Map>());

        auto stat_dir = stat_fn->call({Value::create(dir.string())});
        REQUIRE(stat_dir->is<Map>());
        const auto& dir_map = stat_dir->raw_get<Map>();
        auto dir_type_it = dir_map.find(type_key);
        REQUIRE(dir_type_it != dir_map.end());
        REQUIRE(dir_type_it->second->is<String>());
        CHECK(dir_type_it->second->get<String>() == "directory");

        CHECK_THROWS_AS(
            stat_fn->call({Value::create((dir / "missing").string())}),
            Frost_Recoverable_Error);
    }

    SECTION("symlink")
    {
        auto symlink_fn = get_fs_fn(fs_val, "symlink");

        auto dir = make_temp_dir("fs_symlink");
        BOOST_SCOPE_EXIT_ALL(&dir)
        {
            std::error_code ec;
            std::filesystem::remove_all(dir, ec);
        };

        auto target = dir / "target.txt";
        std::ofstream(target) << "data";
        auto link = dir / "link.txt";

        try
        {
            auto symlink_res = symlink_fn->call(
                {Value::create(target.string()), Value::create(link.string())});
            REQUIRE(symlink_res->is<Null>());
            CHECK(std::filesystem::exists(link));
        }
        catch (const Frost_Recoverable_Error&)
        {
            SUCCEED("symlink not supported on this platform");
        }

        CHECK_THROWS_AS(
            symlink_fn->call({Value::create((dir / "missing" / "t").string()),
                              Value::create((dir / "missing" / "l").string())}),
            Frost_Recoverable_Error);
    }

    SECTION("list_recursively skips permission denied")
    {
        auto list_rec_fn = get_fs_fn(fs_val, "list_recursively");

        auto dir = make_temp_dir("fs_perm");
        auto restricted = dir / "restricted";
        BOOST_SCOPE_EXIT_ALL(&dir, &restricted)
        {
            std::error_code ec;
            if (std::filesystem::exists(restricted))
            {
                std::filesystem::permissions(
                    restricted, std::filesystem::perms::owner_all,
                    std::filesystem::perm_options::replace, ec);
            }
            std::filesystem::remove_all(dir, ec);
        };

        auto visible = dir / "visible.txt";
        std::ofstream(visible) << "ok";
        std::filesystem::create_directories(restricted);
        std::ofstream(restricted / "secret.txt") << "nope";

        std::error_code perm_ec;
        std::filesystem::permissions(restricted, std::filesystem::perms::none,
                                     std::filesystem::perm_options::replace,
                                     perm_ec);
        if (perm_ec)
        {
            SUCCEED("Unable to restrict permissions on this platform");
            return;
        }

        auto rec_val = list_rec_fn->call({Value::create(dir.string())});
        REQUIRE(rec_val->is<Array>());
        const auto& rec_arr = rec_val->raw_get<Array>();
        CHECK(contains_path(rec_arr, visible));
    }
}
