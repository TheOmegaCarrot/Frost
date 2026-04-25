#include "frost/value.hpp"
#include <frost/symbol-table.hpp>

#include <fmt/format.h>

#include <algorithm>

using frst::Symbol_Table;

Symbol_Table::Symbol_Table(const Symbol_Table* failover_table)
    : failover_table_{failover_table}
{
}

Symbol_Table::Symbol_Table(Symbol_Table&& other) noexcept = default;
Symbol_Table& Symbol_Table::operator=(Symbol_Table&& other) noexcept = default;

void Symbol_Table::promote_()
{
    auto m = std::make_unique<map_t>();
    m->reserve(small_size_ * 2);
    for (std::size_t i = 0; i < small_size_; ++i)
        m->try_emplace(std::move(small_[i].first), std::move(small_[i].second));
    map_ = std::move(m);
}

void Symbol_Table::define(const std::string& name, Value_Ptr value)
{
    if (is_small_())
    {
        for (std::size_t i = 0; i < small_size_; ++i)
        {
            if (small_[i].first == name)
                throw Frost_Unrecoverable_Error{fmt::format(
                    "Cannot define {} as it is already defined", name)};
        }

        if (small_size_ < small_capacity)
        {
            small_[small_size_++] = {name, std::move(value)};
            return;
        }

        promote_();
    }

    if (const auto [itr, ok] = map_->try_emplace(name, std::move(value));
        not ok)
        throw Frost_Unrecoverable_Error{
            fmt::format("Cannot define {} as it is already defined", name)};
}

std::optional<frst::Value_Ptr> Symbol_Table::local_lookup_(
    const std::string& name) const
{
    if (is_small_())
    {
        for (std::size_t i = 0; i < small_size_; ++i)
        {
            if (small_[i].first == name)
                return small_[i].second;
        }
        return std::nullopt;
    }

    if (const auto itr = map_->find(name); itr != map_->end())
        return itr->second;
    return std::nullopt;
}

frst::Value_Ptr Symbol_Table::lookup(const std::string& name) const
{
    if (auto result = soft_lookup(name))
        return std::move(result).value();

    throw Frost_Unrecoverable_Error{
        fmt::format("Symbol {} is not defined", name)};
}

std::optional<frst::Value_Ptr> Symbol_Table::soft_lookup(
    const std::string& name) const
{
    for (const auto* table = this; table; table = table->failover_table_)
    {
        if (auto result = table->local_lookup_(name))
            return result;
    }
    return std::nullopt;
}

bool Symbol_Table::local_has_(const std::string& name) const
{
    if (is_small_())
    {
        for (std::size_t i = 0; i < small_size_; ++i)
        {
            if (small_[i].first == name)
                return true;
        }
        return false;
    }

    return map_->find(name) != map_->end();
}

bool Symbol_Table::has(const std::string& name) const
{
    for (const auto* table = this; table; table = table->failover_table_)
    {
        if (table->local_has_(name))
            return true;
    }
    return false;
}

void Symbol_Table::reserve(std::size_t size)
{
    if (size > small_capacity)
    {
        if (is_small_())
            promote_();
        map_->reserve(size);
    }
}

bool Symbol_Table::empty() const
{
    if (is_small_())
        return small_size_ == 0;
    return map_->empty();
}

std::vector<std::string_view> Symbol_Table::names() const
{
    std::vector<std::string_view> result;
    if (is_small_())
    {
        result.reserve(small_size_);
        for (std::size_t i = 0; i < small_size_; ++i)
            result.push_back(small_[i].first);
    }
    else
    {
        result.reserve(map_->size());
        for (const auto& [k, _] : *map_)
            result.push_back(k);
    }
    return result;
}

std::vector<std::string_view> Symbol_Table::deep_names() const
{
    std::vector<std::string_view> result;
    for (const auto* table = this; table; table = table->failover_table_)
    {
        if (table->is_small_())
        {
            for (std::size_t i = 0; i < table->small_size_; ++i)
                result.push_back(table->small_[i].first);
        }
        else
        {
            for (const auto& [k, _] : *table->map_)
                result.push_back(k);
        }
    }
    return result;
}
