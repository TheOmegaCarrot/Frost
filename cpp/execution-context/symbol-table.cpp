#include "frost/value.hpp"
#include <frost/symbol-table.hpp>

#include <fmt/format.h>

#include <algorithm>

using frst::Symbol_Table;

Symbol_Table::Symbol_Table(const Symbol_Table* failover_table)
    : failover_table_{failover_table}
{
}

Symbol_Table::Symbol_Table(const Symbol_Table& other)
    : small_{other.small_}
    , small_size_{other.small_size_}
    , map_{other.map_ ? new map_t{*other.map_} : nullptr}
    , failover_table_{other.failover_table_}
{
}

Symbol_Table::Symbol_Table(Symbol_Table&& other) noexcept
    : small_{std::move(other.small_)}
    , small_size_{other.small_size_}
    , map_{other.map_}
    , failover_table_{other.failover_table_}
{
    other.map_ = nullptr;
    other.small_size_ = 0;
}

Symbol_Table& Symbol_Table::operator=(const Symbol_Table& other)
{
    if (this != &other)
    {
        delete map_;
        small_ = other.small_;
        small_size_ = other.small_size_;
        map_ = other.map_ ? new map_t{*other.map_} : nullptr;
        failover_table_ = other.failover_table_;
    }
    return *this;
}

Symbol_Table& Symbol_Table::operator=(Symbol_Table&& other) noexcept
{
    if (this != &other)
    {
        delete map_;
        small_ = std::move(other.small_);
        small_size_ = other.small_size_;
        map_ = other.map_;
        failover_table_ = other.failover_table_;
        other.map_ = nullptr;
        other.small_size_ = 0;
    }
    return *this;
}

void Symbol_Table::promote_()
{
    auto* m = new map_t{};
    m->reserve(small_size_ * 2);
    for (std::size_t i = 0; i < small_size_; ++i)
        m->try_emplace(std::move(small_[i].first),
                       std::move(small_[i].second));
    map_ = m;
}

void Symbol_Table::define(const std::string& name, Value_Ptr value)
{
    if (is_small_())
    {
        for (std::size_t i = 0; i < small_size_; ++i)
        {
            if (small_[i].first == name)
                throw Frost_Unrecoverable_Error{
                    fmt::format("Cannot define {} as it is already defined",
                                name)};
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

frst::Value_Ptr Symbol_Table::local_lookup_(const std::string& name) const
{
    if (is_small_())
    {
        for (std::size_t i = 0; i < small_size_; ++i)
        {
            if (small_[i].first == name)
                return small_[i].second;
        }
        return nullptr;
    }

    if (const auto itr = map_->find(name); itr != map_->end())
        return itr->second;
    return nullptr;
}

frst::Value_Ptr Symbol_Table::lookup(const std::string& name) const
{
    if (auto result = local_lookup_(name))
        return result;

    if (failover_table_)
        return failover_table_->lookup(name);

    throw Frost_Unrecoverable_Error{
        fmt::format("Symbol {} is not defined", name)};
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
    if (local_has_(name))
        return true;
    if (failover_table_)
        return failover_table_->has(name);
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

const Symbol_Table* Symbol_Table::debug_failover() const
{
    return failover_table_;
}
