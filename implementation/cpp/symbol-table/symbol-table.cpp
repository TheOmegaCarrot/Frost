#include <frost/symbol-table.hpp>

#include <fmt/format.h>

using frst::Symbol_Table;

void Symbol_Table::define(const std::string& name, Value_Ptr value)
{
    if (const auto [itr, ok] = table_.try_emplace(name, std::move(value));
        not ok)
        throw Frost_Error{
            fmt::format("Cannot define {} as it is already defined", name)};
}

frst::Value_Ptr Symbol_Table::lookup(const std::string& name) const
{
    if (const auto itr = table_.find(name); itr != table_.end())
    {
        return itr->second;
    }
    else if (failover_table_)
    {
        return failover_table_->lookup(name);
    }
    else
    {
        throw Frost_Error{fmt::format("Symbol {} is not defined", name)};
    }
}

bool Symbol_Table::has(const std::string& name) const
{
    return (table_.find(name) != table_.end()) || [&] {
        if (failover_table_)
            return failover_table_->has(name);
        return false;
    }();
}
