# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Frost is an interpreted, dynamically-typed functional programming language written in C++26. The interpreter parses source code into an AST, then evaluates it against a symbol table environment.

## Build Commands

CMake build type must be explicitly specified:

```bash
# Configure (Debug)
cmake -S . -B build -DCMAKE_BUILD_TYPE=Debug

# Configure (Release)
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release

# Build
cmake --build build -j4

# Common optional flags:
#   -DBUILD_TESTS=NO   disable tests
```

The main binary is output to `build/frost`.

## Running Tests

Always run tests with `FROST_SKIP_HTTP_TESTS=1` to avoid real network requests:

```bash
# Run all tests
FROST_SKIP_HTTP_TESTS=1 ctest --test-dir build --output-on-failure

# Run a specific test suite (matches by the name in the TEST_CASE macro)
FROST_SKIP_HTTP_TESTS=1 ctest --test-dir build -R <test-name>

# Run a specific test binary directly
FROST_SKIP_HTTP_TESTS=1 ./build/Frost_Tests/<test-name>
```

Test binaries are placed in `build/Frost_Tests/`. Each test file in `cpp/*/tests/` becomes its own executable via the `make_test()` CMake macro.

Integration tests run `.frst` scripts from `integration-tests/` and are discovered automatically by CTest.

## Architecture

**Pipeline:** source text → parser (Lexy grammar) → AST → evaluator → `Value_Ptr`

### Key modules (`cpp/`)

| Module | Purpose |
|---|---|
| `parser/grammar.hpp` | Complete Lexy-based grammar — the source of truth for syntax - large and complex, any work in here should be cautious, and rigorously tested |
| `ast/` | AST node types; each node implements `.evaluate()` or `.execute()` |
| `value/` | Runtime `Value` type and all operators |
| `execution-context/` | Execution context threaded through evaluation, including lexically-scoped variable bindings (`Symbol_Table` with failover chaining) |
| `functions/` | Closures, lambdas, builtins (several builtin modules, and the stdlib) |
| `prelude/prelude.frst` | Standard library written in Frost (loaded at startup) |
| `meta/` | Special functions which depend on the parser, such as `import` |
| `ext/` | Optional extensions isolated from the rest of the Frost codebase (HTTP client, etc.) |

### Value type (`cpp/value/include/frost/value.hpp`)

`Value` is a `shared_ptr`-wrapped `std::variant<Null, Int, Float, Bool, String, Array, Map, Function>`.
- `Null` and `Bool` are singletons (use `Value::null()`, `Value::create(Bool{...})`)
- `Float` rejects NaN and Infinity at construction
- `Map` keys must be non-null primitives (enforced in constructor)
- Use `Value::create(...)` factory methods; direct construction is restricted

### AST evaluation pattern

Each AST node receives an `Execution_Context` (Statements) or `Evaluation_Context` (Expressions) and returns a `Value_Ptr`. The Context contains the symbol table. The parser builds the AST; nodes are defined in `cpp/ast/include/frost/ast/`.
Expressions are incapable of modifying the symbol table by design.

### Adding a builtin function

Each builtin module lives in `cpp/functions/builtins/`. Registration is in `cpp/functions/include/frost/builtin.hpp`. A builtin takes a `std::span<const Value_Ptr>` and returns `Value_Ptr`.

## Agent Instructions

The agent must not modify files without explicit instruction.
The primary duties of the agent are writing/updating unit tests and read-only debugging.
When modifying unit tests, always build and run all tests to ensure correctness.
If a valid test fails, the agent should simply inform the user.

Read before you write — especially for simple tasks. Simple tasks have the highest error rate because the verification steps that naturally happen for complex tasks get skipped. Before writing code, verify the project APIs you intend to call against their actual declarations. Do not assume method names, parameter types, or API shapes from memory.

When writing tests, always consider what the test must prove. Every test should prove some behavior of the code unit being tested.

The agent must not use Python or sed to edit files.

## Frost Syntax Cheat Sheet

This cheat sheet is not exhaustive, but serves as a guard against common mistakes.
`./docs` is available for more complete information.

### Comments

Frost comments are Python-style.

```frost
# This is a comment until the end of the line
this_gets_executed()
```

Comments must only contain ASCII characters.

### `if` / `elif` / `else`

Uses colon, not braces or `then`. Each branch is a single expression.

```frost
if x: 1
elif y: 2
else: 3

if condition: print("yes")   # else: null implicitly
```

Braces are NOT valid branch syntax. Use a `do` block if this is needed:

```frost
if condition: do {
    def x = 1
    x + 2
}
else: 0
```

### `@` threading operator

`a @ f()` means `f(a)`. `a @ f(x)` means `f(a, x)` — the left-hand value is threaded in as the **first** argument.

```frost
[1, 2, 3] @ transform(fn x -> x * 2)   # transform([1, 2, 3], fn x -> x * 2)
value @ tap(print)                       # tap(value, print)
```

### Format strings

`${}` interpolation accepts **identifiers only** — no expressions.

```frost
def name = "world"
$'hello, ${name}'    # ok
$'hello, ${"world"}' # syntax error — string literal is not an identifier
$'result: ${x + 1}'  # syntax error — expressions not allowed
```

### Functions

```frost
fn x -> x + 1                    # single-expression lambda
fn x, y -> x + y                 # multiple parameters
fn -> { def x = 1; x + 2 }       # block body (statements separated by newlines or ;)
fn x -> fn y -> x + y            # curried
fn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)  # named lambda (name available for recursion)
```

`defn name(params) -> body` is sugar for `def name = fn name(params) -> body`:

```frost
defn add(x, y) -> x + y
export defn greet(name) -> $'hello, ${name}'
```

Abbreviated lambdas: `$(expr)` with `$`/`$1`-`$9` placeholders, `$$` for rest args.

```frost
map [1, 2, 3] with $($ * 2)           # [2, 4, 6]
$($1 + $2)                            # fn $1, $2 -> $1 + $2
$($$)                                  # fn ...$$ -> $$
```

Nesting is syntactically valid, but is considered poor practice.

### Truthiness

Only `null` and `false` are falsy. `0`, `""`, `[]`, `{}` are all truthy.

### `and` / `or`

Short-circuit and return the actual value, not a coerced `Bool`.

```frost
null or "default"    # => "default"
42 and "yes"         # => "yes"
false or 0           # => 0
```

### Match expressions

```frost
match value {
    42              => 'the answer',       # literal value pattern
    n is Int        => $'int: ${n}',       # binding with type constraint
    (some_var)      => 'matched var',      # parenthesized: compares against existing variable
    [a, b, ...rest] => a + b,              # array pattern with rest
    {name, age: a}  => a,                  # map pattern (shorthand + explicit)
    {name} as person => person.age,        # map pattern with whole-map binding
    1 | 2 | 3       => 'low',               # alternatives: first match wins
    _ if: guard     => 'guarded',          # guard with `if:` (note the colon)
    _               => 'catch-all'         # discard binding
}
```

- Arms are **comma-separated** (even across newlines). Trailing comma permitted.
- No match is a **runtime error** (not null).
- Bare identifier = binding. `(expr)` = value comparison. Literal = value comparison.
- Map patterns require keys to be **present**; missing key = no match.
- Alternatives (`|`): all branches must bind the **same set of names**. Works at any nesting level.
- Type constraints: `Null`, `Int`, `Float`, `Bool`, `String`, `Array`, `Map`, `Function`, `Primitive`, `Numeric`, `Structured`, `Nonnull`.

### Map / filter / reduce expressions

```frost
map [1, 2, 3] with fn x -> x * 2          # map expression (NOT a function call)
filter [1, 2, 3] with fn x -> x > 1       # filter expression
reduce [1, 2, 3] with fn (acc, x) -> acc + x
```

The functional equivalents (usable in pipelines with `@`) are `transform`, `select`, `fold`.

## Running the Interpreter

```bash
./build/frost script.frst          # run a file
./build/frost                      # REPL
./build/frost -e "print(1 + 2)"    # eval expression
./build/frost -d script.frst       # dump AST
```
