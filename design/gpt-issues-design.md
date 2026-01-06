# Design issues (current)

## Active issues (needs decisions)
None.

## Active todo items (ordered by importance)
None.

## Deferred issues (later in development)
- Operator precedence and associativity are not specified (likely C++-like, to be finalized during implementation).
- Builtin surface area beyond the minimal testing set is not defined; formatting behavior is TBD.
- Explicit numeric parsing/conversion builtin(s) are not specified (needed since string->number coercion is disallowed).
- Newline continuation / ambiguity rules are pending parser implementation.
- String escape sequences, string literal encoding (ASCII vs UTF-8), and future multiline string syntax are not specified.
- Error recovery is deferred (potentially Lua-style `pcall`).
- Multi-value semantics (Lua-style `...` returns) are deferred/likely out of scope for v1; `pack_call`-style apply is the intended alternative.

## Deferred todo items (ordered by importance)
- Specify operator precedence and associativity.
- Define full builtin set and `print`/`format` formatting rules.
- Define newline continuation / ambiguity rules.
- Define error-recovery mechanism, if any.
