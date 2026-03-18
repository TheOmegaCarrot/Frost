#ifndef FROST_EXT_EXTENSIONS_COMMON_HPP
#define FROST_EXT_EXTENSIONS_COMMON_HPP

#include <frost/builtins-common.hpp>

#define DECLARE_EXTENSION(NAME, ...) ::frst::Map make_extension_##NAME()

#define CREATE_EXTENSION(...) return ::frst::Map{__VA_ARGS__};

#endif
