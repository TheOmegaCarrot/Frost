#include <frost/value.hpp>

using namespace frst;

std::optional<Value_Ptr> Value::index_array(const Array& array, Int index)
{
    /*   -3 -2 -1
     *  [ a, b, c ]  The same indexing rules as Python
     *    0  1  2
     */

    const Int len = array.size();

    if (index >= 0 && index < len)
        return array.at(index);

    if (index < 0 && len + index >= 0)
        return array.at(index + len);

    return std::nullopt; // out-of-bounds -> null
}
