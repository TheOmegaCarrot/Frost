# Design understanding (current)

## Syntax
- Top-level is declarations only, using `def`.
- New bindings must use `def`; `=` assigns to an already-bound name.
- Assignment is statement-only (not an expression).
- Function literals require `fn` and use `fn (args) -> { body }`.
- Zero-arg functions are `fn () -> { ... }`.
- `fn { ... }` is a future stretch goal, not v1.
- `return` is a statement only (not allowed in expression positions).
- Newlines can act as ordinary whitespace or as statement separators; semicolons are optional.
- Newlines are separators at top-level and inside `{ ... }` bodies; inside `()`, `[]`, and `%{ ... }` they are treated as whitespace only.
- Comments are `#` line comments only.
- Identifiers are ASCII-only; `_` is a discard placeholder.
- Keywords are fully reserved (`def`, `fn`, `if/elif/else`, `return`, `reduce/map/foreach/with/into`, `true/false/null`, `and/or/not`).

## Expressions and operators
- `if` is an expression form with optional `else`; if `else` is omitted and the condition is false, the result is `null`.
- Logical ops are `and`, `or`, `not` and short-circuit.
- Comparison operators exist (`<`, `<=`, `==`, `!=`, `>=`); precedence/associativity are TBD (likely C++-like, to be finalized during implementation).
- Truthiness: only `false` and `null` are falsy; `0` and `""` are truthy.
- Coercions: implicit numeric conversions allowed; `bool -> number` disallowed; `any -> bool` allowed; `string -> number` allowed.
- `+` is overloaded for numeric addition, string concatenation, array concatenation, and map merge (right-hand value wins on key collision).

## Data structures
- Arrays use `[ ... ]` and allow indexing with `[expr]`.
- Maps use `%{ ... }` with keys:
  - `k1: v` for identifier-like string keys.
  - `[expr]: v` for any other key.
- Dot access `a.k` desugars to `a["k"]` (identifier-like string only).
- `handle` is a distinct, opaque type for interpreter-managed resources (primarily file-like); operated on only via builtins.
- Sockets or directories, if ever added, are treated as file-like handles (Linux-focused implementation).
- Subprocess management can be represented via file handles for standard IO streams.
- Time handling is expected via a `now` builtin returning an integer.
- Concurrency, database connections, networking, and UI/media are likely out of scope.
- Handle equality is identity-based.
- `type(x)` returns `"handle"` for handles (no subtype tags).

## Higher-order forms
- `reduce`:
  - Arrays: callback receives `(acc, item)`.
  - Maps: callback receives `(acc, k, v)`.
  - Reduction over an empty collection without `init` yields `null`.
- `map` (v1):
  - Array -> array mapping is allowed.
  - Map -> map mapping is allowed; callback must evaluate to a map, which is merged into an accumulator.
  - Map traversal order is unspecified; duplicate keys are last-wins and nondeterministic.
  - Array -> map mapping via `map <array> into map ...` is deferred (not parsed in v1).
  - Map -> array mapping is disallowed.
- `foreach` iterates with callbacks similar to `map`/`reduce`.
- `foreach` always returns `null`.

## Recursion and binding
- Recursive value definitions are an error.
- Recursive function definitions are allowed; the name is bound after the function value is evaluated.

## Functions and blocks
- Function bodies are sequences of statements separated by newlines or semicolons.
- Expression statements are allowed; the last expression in a function body is the implicit return value if no explicit `return` is executed.
- Functions return a single value.
- If a function body ends with a non-expression statement (e.g., `def` or assignment), the implicit return value is `null`.

## Builtins and examples
- Builtin functions are deferred until late in v1 development; a minimal set will exist for early testing.
- Minimal testing set: `print`, `format`, `tostring`, `assert`, `type`, `len`.
- Variadic calls are required (at least for `print`/`format`).
- User-defined functions support variadic arguments; syntax is TBD.
- Formatting behavior and the full builtin set are not finalized.
