#ifndef FROST_IMPORT_HPP
#define FROST_IMPORT_HPP

#include <frost/symbol-table.hpp>

#include <filesystem>
#include <vector>

namespace frst
{
std::vector<std::filesystem::path> env_module_path();
void inject_import(Symbol_Table& table,
                   const std::vector<std::filesystem::path>& search_path);
} // namespace frst

#endif
