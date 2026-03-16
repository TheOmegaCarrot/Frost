#ifndef FROST_IMPORT_HPP
#define FROST_IMPORT_HPP

#include <frost/symbol-table.hpp>

#include <filesystem>
#include <vector>

namespace frst
{

class Backtrace_State;

std::vector<std::filesystem::path> env_module_path();
void inject_import(Symbol_Table& table,
                   const std::vector<std::filesystem::path>& search_path,
                   Backtrace_State* bt = nullptr);
} // namespace frst

#endif
