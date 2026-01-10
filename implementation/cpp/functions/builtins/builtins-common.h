#ifndef FROST_BUILTINS_COMMON_HPP
#define FROST_BUILTINS_COMMON_HPP

#include <frost/builtin.hpp>
#include <frost/symbol-table.hpp>

#define INJECT(NAME, MIN_ARITY, MAX_ARITY)                                     \
    table.define(#NAME,                                                        \
                 Value::create(Function{std::make_shared<Builtin>(             \
                     &NAME, #NAME,                                             \
                     Builtin::Arity{.min = MIN_ARITY, .max = MAX_ARITY})}))

#define INJECT_V(NAME, MIN_ARITY) INJECT(NAME, MIN_ARITY, std::nullopt)

#endif
