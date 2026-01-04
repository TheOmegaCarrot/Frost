#ifndef FROST_SYMBOL_TABLE_HPP
#define FROST_SYMBOL_TABLE_HPP

#include <frost/value.hpp>

#include <optional>
#include <string>
#include <unordered_map>

namespace frst
{

class Symbol_Table
{
  public:
    Symbol_Table() = default;
    Symbol_Table(const Symbol_Table* failover_table)
        : failover_table_{failover_table}
    {
    }
    Symbol_Table(const Symbol_Table&) = default;
    Symbol_Table(Symbol_Table&&) = default;
    Symbol_Table& operator=(const Symbol_Table&) = default;
    Symbol_Table& operator=(Symbol_Table&&) = default;
    ~Symbol_Table() = default;

    // Bind a value to a name within this symbol table
    // Returns nullopt on success, or a useful error message on failure
    std::optional<std::string> define(std::string name, Value_Ptr value);

    // Looks up a value by name within the symbol table
    // If not found in this table, attempty to lookup
    //      in the failover table, if present
    // Returns nullopt if the name is not defined
    std::optional<Value_Ptr> lookup(const std::string& name) const;

  private:
    std::unordered_map<std::string, Value_Ptr> table_;
    const Symbol_Table* failover_table_;
};

} // namespace frst

#endif
