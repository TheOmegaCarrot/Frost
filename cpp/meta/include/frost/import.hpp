#ifndef FROST_IMPORT_HPP
#define FROST_IMPORT_HPP

#include <frost/stdlib.hpp>
#include <frost/symbol-table.hpp>

#include <filesystem>
#include <mutex>
#include <vector>

namespace frst
{

namespace import
{
using search_path_t = std::vector<std::filesystem::path>;

using import_cache_map_t =
    std::flat_map<std::filesystem::path, std::optional<Value_Ptr>>;
using import_cache_t = std::shared_ptr<import_cache_map_t>;

using import_mutex_t = std::shared_ptr<std::recursive_mutex>;

using import_stack_t = std::vector<std::string>;
} // namespace import

std::vector<std::filesystem::path> env_module_path();
void inject_import(Symbol_Table& table,
                   const import::search_path_t& search_path,
                   const Symbol_Table& root_table,
                   std::shared_ptr<Stdlib_Registry> stdlib = nullptr,
                   import::import_cache_t import_cache =
                       std::make_shared<import::import_cache_map_t>(),
                   import::import_mutex_t import_mutex =
                       std::make_shared<std::recursive_mutex>(),
                   const import::import_stack_t& import_stack = {
                       "main script"});
} // namespace frst

#endif
