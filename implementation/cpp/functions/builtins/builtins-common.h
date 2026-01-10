#ifndef FROST_BUILTINS_COMMON_HPP
#define FROST_BUILTINS_COMMON_HPP

#define INJECT(NAME, MAX_ARITY)                                                \
    table.define(#NAME, Value::create(Function{std::make_shared<Builtin>(      \
                            &NAME, #NAME, MAX_ARITY)}))

#endif
