#include <frost/builtins-common.hpp>
#include <frost/import.hpp>
#include <frost/parser.hpp>
#include <frost/prelude.hpp>
#include <frost/symbol-table.hpp>

#include <filesystem>
#include <flat_map>

#include <boost/algorithm/string/replace.hpp>
#include <fmt/format.h>

namespace frst
{

std::vector<std::filesystem::path> env_module_path()
{
    if (const char* extra_path = std::getenv("FROST_MODULE_PATH"))
    {
        return std::string{extra_path}
               | std::views::split(':')
               | std::views::transform([]<typename T>(T&& component) {
                     return std::filesystem::path{std::string{
                         std::from_range, std::forward<T>(component)}};
                 })
               | std::ranges::to<std::vector<std::filesystem::path>>();
    }

    return {};
}

namespace
{

struct Importer
{
    std::vector<std::filesystem::path> search_path;

    std::flat_map<std::string, Value_Ptr> import_cache;

    Value_Ptr operator()(builtin_args_t args)
    {
        REQUIRE_ARGS("import", PARAM("module", TYPES(String)));

        auto module_spec = GET(0, String);

        if (auto cache_hit = import_cache.find(module_spec);
            cache_hit != import_cache.end())
        {
            return cache_hit->second;
        }

        std::filesystem::path module_path =
            boost::replace_all_copy(module_spec, ".", "/") + ".frst";

        for (const auto& search_dir : search_path)
        {
            auto prospective_module_file = search_dir / module_path;
            if (std::filesystem::is_regular_file(prospective_module_file))
            {
                return do_import(module_spec, prospective_module_file);
            }
        }

        throw Frost_Recoverable_Error{
            fmt::format("Could not resolve import {}", module_spec)};
    }

    Value_Ptr do_import(const std::string& module_spec,
                        const std::filesystem::path& module_file)
    {
        auto parse_result = parse_file(module_file);

        if (not parse_result)
        {
            throw Frost_Recoverable_Error{fmt::format(
                "Error importing module {} (resolved to {}):\n{}", module_spec,
                module_file.native(), parse_result.error())};
        }

        Symbol_Table isolated_table;
        inject_builtins(isolated_table);
        inject_prelude(isolated_table);
        isolated_table.define("imported", Value::create(true));

        std::vector<std::filesystem::path> child_search_path{
            module_file.parent_path(),
        };

        child_search_path.append_range(env_module_path());

        inject_import(isolated_table, child_search_path);

        Map imported;
        for (const auto& statement : parse_result.value())
        {
            if (const auto& exported = statement->execute(isolated_table))
                imported.insert_range(exported.value());
        }

        auto import_result = Value::create(std::move(imported));
        import_cache.emplace(module_spec, import_result);
        return import_result;
    }
};

} // namespace

void inject_import(Symbol_Table& table,
                   const std::vector<std::filesystem::path>& search_path)
{
    table.define("import", Value::create(Function{std::make_shared<Builtin>(
                               Importer{std::move(search_path), {}}, "import",
                               Builtin::Arity{.min = 1, .max = 1})}));
}
} // namespace frst
