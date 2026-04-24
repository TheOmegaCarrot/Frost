#ifndef FROST_SYMBOL_TABLE_HPP
#define FROST_SYMBOL_TABLE_HPP

#include <frost/value.hpp>

#include <ranges>
#include <string>

#include <ankerl/unordered_dense.h>

namespace frst
{

class Symbol_Table
{
  public:
    using map_type = ankerl::unordered_dense::map<std::string, Value_Ptr>;
    Symbol_Table() = default;
    Symbol_Table(const Symbol_Table* failover_table);
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

    virtual void reserve(std::size_t size);

    bool empty() const;
    auto names() const { return table_ | std::views::keys; }

    const Symbol_Table* debug_failover() const;

  private:
    map_type table_;
    const Symbol_Table* failover_table_ = nullptr;
};

} // namespace frst

#endif
