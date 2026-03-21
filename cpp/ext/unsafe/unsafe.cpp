#include <frost/extensions-common.hpp>

namespace frst
{
namespace unsafe
{

BUILTIN(identity)
{
    return Value::create(
        Int{reinterpret_cast<std::intptr_t>(args.at(0).get())});
}

BUILTIN(same)
{
    return Value::create(args.at(0).get() == args.at(1).get());
}

} // namespace unsafe

DECLARE_EXTENSION(unsafe)
{
    using namespace unsafe;
    CREATE_EXTENSION(ENTRY(identity, 1), ENTRY(same, 2));
}
} // namespace frst
