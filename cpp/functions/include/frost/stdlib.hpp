#ifndef FROST_FUNCTIONS_STDLIB_HPP
#define FROST_FUNCTIONS_STDLIB_HPP

#include <frost/value.hpp>

#include <span>
#include <string_view>

namespace frst
{

class Stdlib_Registry;

class Stdlib_Registry_Builder
{
  public:
    using module_path_t = std::span<const std::string_view, 2>;

    void register_module(module_path_t path, Value_Ptr contents);
    Stdlib_Registry build() &&;

  private:
    std::flat_map<std::string, Map> staged_;
};

class Stdlib_Registry
{
  public:
    Stdlib_Registry(const Stdlib_Registry&) = delete;
    Stdlib_Registry(Stdlib_Registry&&) = default;
    Stdlib_Registry& operator=(const Stdlib_Registry&) = delete;
    Stdlib_Registry& operator=(Stdlib_Registry&&) = default;
    ~Stdlib_Registry() = default;

    std::optional<Value_Ptr> lookup_module(std::string_view path) const;

  private:
    friend class Stdlib_Registry_Builder;
    explicit Stdlib_Registry(Value_Ptr root)
        : root_(std::move(root))
    {
    }
    Value_Ptr root_;
};

#define X_STDLIB_MODULES                                                       \
    X(cli)                                                                     \
    X(encoding)                                                                \
    X(fs)                                                                      \
    X(io)                                                                      \
    X(json)                                                                    \
    X(math)                                                                    \
    X(os)                                                                      \
    X(random)                                                                  \
    X(regex)                                                                   \
    X(string)

#define X(module) void register_module_##module(Stdlib_Registry_Builder&);

X_STDLIB_MODULES

#undef X

inline void register_stdlib(Stdlib_Registry_Builder& builder)
{
#define X(module) register_module_##module(builder);

    X_STDLIB_MODULES

#undef X
}

} // namespace frst

#endif
