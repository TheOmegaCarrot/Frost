#ifndef FROST_SYMBOL_TABLE_HPP
#define FROST_SYMBOL_TABLE_HPP

#include <frost/value.hpp>

#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include <ankerl/unordered_dense.h>

namespace frst
{

class Symbol_Table
{
  public:
    static constexpr std::size_t small_capacity = 8;

    Symbol_Table() = default;
    Symbol_Table(const Symbol_Table* failover_table);
    Symbol_Table(const Symbol_Table&);
    Symbol_Table(Symbol_Table&&) noexcept;
    Symbol_Table& operator=(const Symbol_Table&);
    Symbol_Table& operator=(Symbol_Table&&) noexcept;
    virtual ~Symbol_Table() = default;

    // Bind a value to a name within this symbol table
    // Throws on redefinition error
    virtual void define(const std::string& name, Value_Ptr value);

    // Looks up a value by name within the symbol table
    // If not found in this table, attempts to lookup
    //      in the failover table, if present
    // Throws on failed lookup
    virtual Value_Ptr lookup(const std::string& name) const;

    // Check if a name is defined within the symbol table
    virtual bool has(const std::string& name) const;

    virtual void reserve(std::size_t size);

    bool empty() const;
    std::vector<std::string_view> names() const;
    std::vector<std::string_view> deep_names() const;

  private:
    using entry_t = std::pair<std::string, Value_Ptr>;
    using small_storage_t = std::array<entry_t, small_capacity>;
    using map_t =
        ankerl::unordered_dense::map<std::string, Value_Ptr>;

    small_storage_t small_{};
    std::size_t small_size_ = 0;
    map_t* map_ = nullptr;
    const Symbol_Table* failover_table_ = nullptr;

    bool is_small_() const { return map_ == nullptr; }
    void promote_();

    Value_Ptr local_lookup_(const std::string& name) const;
    bool local_has_(const std::string& name) const;
};

} // namespace frst

#endif
