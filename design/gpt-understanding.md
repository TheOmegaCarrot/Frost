# Design understanding (current)

## Syntax
- Top-level is declarations only, using `def`.
- Function literals require `fn` and use `fn (args) -> { body }`.
- Zero-arg functions are `fn () -> { ... }`.
- `fn { ... }` is a future stretch goal, not v1.
- `return` is a statement only (not allowed in expression positions).
- Newlines can act as ordinary whitespace or as statement separators; semicolons are optional.
- Comments are `#` line comments only.
- Identifiers are ASCII-only; `_` is a discard placeholder.
- Keywords are fully reserved (`def`, `fn`, `if/elif/else`, `return`, `reduce/map/foreach/with/into`, `true/false/null`, `and/or/not`).

## Expressions and operators
- `if` is an expression form: `if cond: expr elif ... else: expr`.
- Logical ops are `and`, `or`, `not` and short-circuit.
- Comparison operators exist (`<`, `<=`, `==`, `!=`, `>=`); precedence is TBD.
- Truthiness: only `false` and `null` are falsy; `0` and `""` are truthy.
- Coercions: implicit numeric conversions allowed; `bool -> number` disallowed; `any -> bool` allowed; `string -> number` allowed.

## Data structures
- Arrays use `[ ... ]` and allow indexing with `[expr]`.
- Maps use `%{ ... }` with keys:
  - `k1: v` for identifier-like string keys.
  - `[expr]: v` for any other key.
- Dot access `a.k` desugars to `a["k"]` (identifier-like string only).

## Higher-order forms
- `reduce`:
  - Arrays: callback receives `(acc, item)`.
  - Maps: callback receives `(acc, k, v)`.
- `map`:
  - Array -> array mapping is allowed.
  - Map -> map mapping is allowed; callback returns a map that is merged into an accumulator.
  - Map traversal order is unspecified; duplicate keys are last-wins and nondeterministic.
  - Array -> map mapping uses `map <array> into map ...` (may be omitted in early versions).
  - Map -> array mapping is disallowed.
- `foreach` iterates with callbacks similar to `map`/`reduce`.

## Recursion and binding
- Recursive value definitions are an error.
- Recursive function definitions are allowed; the name is bound after the function value is evaluated.

## Builtins and examples
- `print` is a builtin function; formatting behavior is not specified.
- `tostring` appears in examples; builtin set is not finalized.
