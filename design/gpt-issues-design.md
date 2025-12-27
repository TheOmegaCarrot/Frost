# Design issues (current)

## Active issues (needs decisions)
None.

## Active todo items (ordered by importance)
None.

## Deferred issues (later in development)
- Operator precedence and associativity are not specified (likely C++-like, to be finalized during implementation).
- Builtin surface area beyond the minimal testing set is not defined; formatting behavior is TBD.
- Variadic function syntax (including user-defined functions) is not specified.
- String escape sequences and future multiline string syntax are not specified.
- Out-of-bounds array assignment null-fill is tentative and may change.
- Error recovery is deferred (potentially Lua-style `pcall`).

## Deferred todo items (ordered by importance)
- Specify operator precedence and associativity.
- Define full builtin set and `print`/`format` formatting rules.
- Specify variadic function syntax.
- Define error-recovery mechanism, if any.
