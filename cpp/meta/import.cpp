#include "frost/exceptions.hpp"
#include <frost/backtrace.hpp>
#include <frost/builtins-common.hpp>
#include <frost/ext.hpp>
#include <frost/import.hpp>
#include <frost/parser.hpp>
#include <frost/prelude.hpp>
#include <frost/symbol-table.hpp>

#include <filesystem>
#include <flat_map>

#include <boost/algorithm/string/replace.hpp>
#include <boost/scope_exit.hpp>
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
    import::search_path_t search_path;

    // Pre-built table with builtins, meta, and prelude.
    // Each import gets a child scope (via failover pointer) rather than
    // rebuilding from scratch.
    const Symbol_Table* root_table;

    // key: resolved filepath
    // value: cached import result (nullopt while import is in-progress (for
    // cycle detection))
    import::import_cache_t import_cache;

    // Serializes all file-based imports interpreter-wide. Recursive because
    // transitive imports (A imports B imports C) re-enter do_import on the
    // same thread.
    import::import_mutex_t import_mutex;

    import::import_stack_t import_stack;

    std::shared_ptr<Stdlib_Registry> stdlib;

    Value_Ptr operator()(builtin_args_t args)
    {
        REQUIRE_ARGS("import", PARAM("module", TYPES(String)));

        auto module_spec = GET(0, String);

        if (module_spec.empty())
            throw Frost_Recoverable_Error{
                "import requires a non-empty module name"};

        if (stdlib)
        {
            if (auto result = stdlib->lookup_module(module_spec))
                return result.value();
        }

        std::filesystem::path module_path =
            boost::replace_all_copy(module_spec, ".", "/") + ".frst";

        for (const auto& search_dir : search_path)
        {
            auto prospective_module_file = search_dir / module_path;
            if (std::filesystem::is_regular_file(prospective_module_file))
            {
                return do_import(module_spec, std::filesystem::canonical(
                                                  prospective_module_file));
            }
        }

        throw Frost_Recoverable_Error{
            fmt::format("Could not resolve import {}", module_spec)};
    }

    Value_Ptr do_import(const std::string& module_spec,
                        const std::filesystem::path& module_file)
    {
        std::lock_guard lock{*import_mutex};

        import_stack.push_back(module_spec);
        BOOST_SCOPE_EXIT_ALL(&)
        {
            import_stack.pop_back();
        };

        if (auto cache_hit = import_cache->find(module_file);
            cache_hit != import_cache->end())
        {
            if (not cache_hit->second.has_value())
            {
                throw Frost_Recoverable_Error{fmt::format(
                    "Import cycle ({})", fmt::join(import_stack, " -> "))};
            }

            return cache_hit->second.value();
        }

        // mark the import as in-progress for cycle detection
        import_cache->emplace(module_file, std::nullopt);
        BOOST_SCOPE_EXIT_ALL(&)
        {
            // If the import fails, remove the import-in-progress marker
            if (import_cache->at(module_file) == std::nullopt)
                import_cache->erase(module_file);
        };

        auto guard = make_frame_guard("Import Boundary ({})", module_spec);

        auto parse_result = parse_file(module_file);

        if (not parse_result)
        {
            throw Frost_Recoverable_Error{fmt::format(
                "Error importing module {} (resolved to {}):\n{}", module_spec,
                module_file.native(), parse_result.error())};
        }

        Symbol_Table isolated_table(root_table);
        isolated_table.define("imported", Value::create(true));

        Execution_Context isolated_ctx{.symbols = isolated_table};

        std::vector<std::filesystem::path> child_search_path{
            module_file.parent_path(),
        };

        child_search_path.append_range(env_module_path());

        inject_import(isolated_table, child_search_path, *root_table, stdlib,
                      import_cache, import_mutex, import_stack);

        for (const auto& statement : parse_result.value())
        {
            statement->execute(isolated_ctx);
        }

        auto exported_names =
            parse_result.value()
            | std::views::transform(
                [](const ast::Statement::Ptr& stmt)
                    -> std::generator<ast::AST_Node::Symbol_Action> {
                    co_yield std::ranges::elements_of(stmt->symbol_sequence());
                })
            | std::views::join
            | std::views::filter([](const ast::AST_Node::Symbol_Action& sym) {
                  return sym.visit(Overload{
                      [](const ast::AST_Node::Definition& def) {
                          return def.exported;
                      },
                      [](const ast::AST_Node::Usage&) {
                          return false;
                      },
                  });
              })
            | std::views::transform(
                [](const ast::AST_Node::Symbol_Action& sym) {
                    return sym.visit([](const auto& action) {
                        return action.name;
                    });
                });

        Map imported;
        for (const std::string& name : exported_names)
        {
            try
            {
                // names can only be defined once, so collisions should not be
                // possible here
                imported.emplace(Value::create(auto{name}),
                                 isolated_table.lookup(name));
            }
            catch (const std::exception& e)
            {
                // if the name could not be found, then there is a bug in the
                // interpreter, because the names were pulled right from the
                // symbol_sequence
                throw Frost_Interpreter_Error{fmt::format(
                    "Error looking up exported symbol {}: {}", name, e.what())};
            }
        }

        auto import_result = Value::create(Value::trusted, std::move(imported));
        import_cache->insert_or_assign(module_file, import_result);
        return import_result;
    }
};

} // namespace

void inject_import(Symbol_Table& table,
                   const import::search_path_t& search_path,
                   const Symbol_Table& root_table,
                   std::shared_ptr<Stdlib_Registry> stdlib,
                   import::import_cache_t import_cache,
                   import::import_mutex_t import_mutex,
                   const import::import_stack_t& import_stack)
{
    table.define(
        "import",
        Value::create(Function{std::make_shared<Builtin>(
            Importer{.search_path = std::move(search_path),
                     .root_table = &root_table,
                     .import_cache = import_cache,
                     .import_mutex = import_mutex,
                     .import_stack = import_stack,
                     .stdlib = std::move(stdlib)},
            "import")}));
}
} // namespace frst
