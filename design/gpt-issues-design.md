# Design issues (current)

## Active issues (needs decisions)
- Documentation mismatch: `design.frst` uses `say_hi = fn () -> { ... }` without `def` inside `main`; should be `def` under the current rules.

## Active todo items (ordered by importance)
- Update `design.frst` to match the `def`-required rule for new bindings.

## Deferred issues (later in development)
- Operator precedence and associativity are not specified (likely C++-like, to be finalized during implementation).
- Builtin surface area beyond the minimal testing set is not defined; formatting behavior is TBD.
- Variadic function syntax (including user-defined functions) is not specified.

## Deferred todo items (ordered by importance)
- Specify operator precedence and associativity.
- Define full builtin set and `print`/`format` formatting rules.
- Specify variadic function syntax.
