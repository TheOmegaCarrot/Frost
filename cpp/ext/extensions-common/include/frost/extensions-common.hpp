#ifndef FROST_EXT_EXTENSIONS_COMMON_HPP
#define FROST_EXT_EXTENSIONS_COMMON_HPP

#include <frost/builtins-common.hpp>

#define REGISTER_EXTENSION(NAME, ...) REGISTRY_MODULE(ext, NAME, __VA_ARGS__)

#endif
