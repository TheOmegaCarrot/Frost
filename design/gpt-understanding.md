# Design understanding (current)

## Syntax
- Top-level uses the same statement rules as function bodies; expressions are allowed at top-level.
- New bindings must use `def`; reassignment/mutation syntax is not supported.
- Function literals require `fn` and use `fn (args) -> { body }`.
- Zero-arg functions are `fn () -> { ... }`.
- `fn { ... }` is a future stretch goal, not v1.
- There is no `return` statement; functions return the value of the last expression in their body (or `null` if none).
- Newlines can act as ordinary whitespace or as statement separators; semicolons are optional.
- Newlines are separators at top-level and inside `{ ... }` bodies.
- Inside `()`, `[]`, and `%{ ... }`, newlines are treated as whitespace only.
- `if` expressions may span arbitrary newlines between tokens (`if`/`elif`/`else`, condition, `:`, branch), and are parsed as a single expression despite newline separators.
- Comments are `#` line comments only.
- Identifiers are ASCII-only; `_` is a discard placeholder.
- Keywords are fully reserved (`def`, `fn`, `if/elif/else`, `reduce/map/foreach/with/into`, `true/false/null`, `and/or/not`).
- `return` is not a keyword and may be used as an identifier.
- Strings use double quotes; escape sequences are TBD (likely C-like). Multiline string literals are not supported with `"` (future syntaxes may add multiline strings).
- String literal encoding is implementation-dependent; UTF-8 will be supported if it is easy, but it is not a priority.

## Expressions and operators
- `if` is an expression form with optional `else`; if `else` is omitted and the condition is false, the result is `null`.
- The branch expression may appear on the same line as `:` or on the following line; same-line is idiomatic for short expressions.
- Logical ops are `and`, `or`, `not` and short-circuit.
- `and`/`or` return one of their operands (Lua-style), not necessarily a boolean.
- Comparison operators exist (`<`, `<=`, `==`, `!=`, `>=`); precedence/associativity are TBD (likely C++-like, to be finalized during implementation).
- Truthiness: only `false` and `null` are falsy; `0` and `""` are truthy.
- Coercions: implicit numeric conversions allowed; `bool -> number` disallowed; `any -> bool` allowed; `string -> number` allowed.
- Invalid string-to-number coercions raise a runtime error.
- V1 error handling is immediate abort (no recovery). Error recovery (e.g., Lua-style `pcall`) is deferred.
- `+` is overloaded for numeric addition, string concatenation, array concatenation, and map merge (right-hand value wins on key collision).
- Array concatenation and map merge always produce new values (no in-place mutation).
- Array concatenation requires both operands to be arrays; `array + non-array` is an error (use `arr + [value]` to append).
- Map merge requires both operands to be maps; `map + non-map` is a runtime error.
- String concatenation requires both operands to be strings; no implicit `tostring` coercion for `+` (string + non-string is a runtime error).
- Any other mixed-type use of `+` is a runtime error.
- `==`/`!=` use identity equality for arrays, maps, and functions.
- UFCS: `lhs @ func(args...)` is equivalent to `func(lhs, args...)`.
- `@` binds tightly and is left-associative; `a@f()@g()` is equivalent to `g(f(a))`.
- The RHS must be a call; `a@b` is a syntax error.
- `a@f().g` is equivalent to `(a@f()).g`; `a@f()[0]` is equivalent to `f(a)[0]`.

## Types
- Value kinds include `null`, `bool`, numbers (integer/float literals), `string`, `array`, `map`, and `function`.
- Numeric literals are plain decimal integers/floats (no exponent notation or underscores). Integer bounds are int64; float bounds are IEEE 754 double. Exact overflow/parse behavior follows the implementation's string-to-number conversion.

## Data structures
- Arrays use `[ ... ]` and allow indexing with `[expr]`.
- Arrays are 0-indexed.
- Maps use `%{ ... }` with keys:
  - `k1: v` for identifier-like string keys.
  - `[expr]: v` for any other key.
- Dot access `a.k` desugars to `a["k"]` (identifier-like string only).
- Out-of-bounds array access and missing map keys return `null`.
- Arrays and maps are immutable after construction; index assignment is not supported.
- Map literal duplicate keys:
  - Duplicate literal keys are an error.
  - Duplicate computed keys are allowed but have unspecified winner (avoid).
- Map keys may be any value (including arrays/maps/functions). Arrays/maps/functions use identity equality as keys.
- Floating-point keys are allowed but equality is imprecise; users should not rely on float keys for stable lookup.
- No `handle` type: file I/O is expected via builtins that take a filename.
- Time handling is expected via a `now` builtin returning an integer.
- Concurrency, database connections, networking, and UI/media are likely out of scope.

## Higher-order forms
- `reduce`:
  - Arrays: callback receives `(acc, item)`.
  - Maps: callback receives `(acc, k, v)`.
- Without `init`, arrays seed `acc` with the first element and start from the second (foldl1-style).
- Map reductions require `init`.
- Reduction over an empty collection without `init` yields `null`.
- Reduction over an empty collection with `init` yields the `init` value.
- Reductions are expected to use associative operations to avoid order-dependent results.
- `map` (v1):
  - Array callbacks receive `(item)`.
  - Map callbacks receive `(k, v)`.
  - Array -> array mapping is allowed.
  - Map -> map mapping is allowed; callback must evaluate to a map, which is merged into an accumulator.
  - Map traversal order is unspecified; duplicate keys are last-wins and nondeterministic.
  - Array -> map mapping via `map <array> into map ...` is deferred (not parsed in v1).
  - Map -> array mapping is disallowed.
- `foreach` iterates with callbacks similar to `map`/`reduce` (arrays: `(item)`, maps: `(k, v)`).
- `foreach` always returns `null`.

## Recursion and binding
- Self-referential non-function definitions are errors (e.g., `def x = x + 1`).
- Recursive function definitions are allowed; the function name is bound before its body is evaluated at call time, so self-recursion works without special syntax.
- Functions are first-class and capture their lexical environment by reference (late-bound names become visible once bound).
- Bindings are immutable.
- Forward references to later `def` in the same scope are errors (except for self-recursive function definitions).

## Functions and blocks
- Function bodies are sequences of statements separated by newlines or semicolons.
- Expression statements are allowed; the last expression in a function body is the return value.
- Functions return a single value.
- If a function body ends with a `def` statement (no trailing expression), the return value is `null`.
- Empty function bodies (or bodies with only comments/`def`) return `null`.
- Statements include `def` and expression statements.
- Function calls: too few arguments fill missing parameters with `null`; too many arguments are an error (unless the function is variadic).
- `args` is a predefined global array of strings equivalent to process `argv` (including the script name at `args[0]`).
- `main` has no special role; it is an ordinary function unless user code calls it.
- Top-level expression results are discarded; top-level expressions are for side-effects only.

## Builtins and examples
- Builtin functions are deferred until late in v1 development; a minimal set will exist for early testing.
- Minimal testing set: `print`, `format`, `tostring`, `assert`, `type`, `len`, `pack_call`.
- `pack_call(fun, args)` applies a function to an array of arguments (no true multi-value semantics). `args` must be an array; normal arity rules apply.
- `pack_call` with a non-array `args` is a runtime error.
- Variadic calls are required (at least for `print`/`format`).
- User-defined functions support variadic arguments using `...rest` in the parameter list (rest must be last). Extra args are collected into an array parameter (no special varargs type).
- Formatting behavior and the full builtin set are not finalized.
- The `init:` keyword-argument syntax is a special-case for `reduce`, not a general named-argument feature.
