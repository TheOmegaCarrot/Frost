#ifndef FROST_SYMBOL_TABLE_HPP
#define FROST_SYMBOL_TABLE_HPP

#include <frost/value.hpp>

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
    virtual ~Symbol_Table() = default;

    // Bind a value to a name within this symbol table
    // Throws on redefinition error
    virtual void define(const std::string& name, Value_Ptr value);

    // Looks up a value by name within the symbol table
    // If not found in this table, attempty to lookup
    //      in the failover table, if present
    // Throws on failed lookup
    virtual Value_Ptr lookup(const std::string& name) const;

    // Check if a name is defined within the symbol table
    virtual bool has(const std::string& name) const;

    const auto& debug_table() const
    {
        return table_;
    }

    const auto* debug_failover() const
    {
        return failover_table_;
    }

  private:
    std::unordered_map<std::string, Value_Ptr> table_;
    const Symbol_Table* failover_table_ = nullptr;
};

} // namespace frst

#endif
