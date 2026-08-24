#ifndef FROST_FUNCTIONS_DATA_BUILTIN_HPP
#define FROST_FUNCTIONS_DATA_BUILTIN_HPP

#include <frost/builtin.hpp>

#include <concepts>

namespace frst
{

template <typename Data>
class Data_Builtin final : public Builtin
{
  public:
    using Ptr = std::shared_ptr<Data_Builtin>;

    Data_Builtin() = delete;
    Data_Builtin(const Data_Builtin&) = delete;
    Data_Builtin(Data_Builtin&&) = delete;
    Data_Builtin& operator=(const Data_Builtin&) = delete;
    Data_Builtin& operator=(Data_Builtin&&) = delete;
    ~Data_Builtin() final = default;

    template <typename Data_>
        requires std::constructible_from<Data, Data_>
    Data_Builtin(function_t function, std::string name, Data_&& data)
        : Builtin(std::move(function), std::move(name))
        , data_{std::forward<Data_>(data)}
    {
    }

    const Data& data() const
    {
        return data_;
    }

  private:
    Data data_;
};

} // namespace frst

#endif
