#ifndef FROST_FUNCTIONS_STDLIB_HPP
#define FROST_FUNCTIONS_STDLIB_HPP

#include <frost/value.hpp>

namespace frst
{
class Stdlib_Registry
{

  public:
    Stdlib_Registry() = default;
    Stdlib_Registry(const Stdlib_Registry&) = delete;
    Stdlib_Registry(Stdlib_Registry&&) = default;
    Stdlib_Registry& operator=(const Stdlib_Registry&) = delete;
    Stdlib_Registry& operator=(Stdlib_Registry&&) = default;
    ~Stdlib_Registry() = default;

    void register_module(std::string module_name, Value_Ptr module_contents)
    {
        registry_.emplace(std::move(module_name), std::move(module_contents));
    }

    std::optional<Value_Ptr> lookup_module(const std::string& module_name)
    {
        auto itr = registry_.find(module_name);

        if (itr == registry_.end())
            return std::nullopt;

        return itr->second;
    }

  private:
    std::flat_map<std::string, Value_Ptr> registry_;
};

#define X_STDLIB_MODULES                                                       \
    X(json)                                                                    \
    X(regex)

#define X(module) void register_module_##module(Stdlib_Registry&);

X_STDLIB_MODULES

#undef X

inline Stdlib_Registry create_stdlib()
{
    Stdlib_Registry registry;

#define X(module) register_module_##module(registry);

    X_STDLIB_MODULES

#undef X

    return registry;
}

} // namespace frst

#endif
