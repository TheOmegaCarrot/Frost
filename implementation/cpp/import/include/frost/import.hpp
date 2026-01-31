#ifndef FROST_IMPORT_HPP
#define FROST_IMPORT_HPP

#include <frost/symbol-table.hpp>

#include <filesystem>
#include <vector>

namespace frst
{
void inject_import(Symbol_Table& table,
                   const std::vector<std::filesystem::path>& search_path);
} // namespace frst

#endif
