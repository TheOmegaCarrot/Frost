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
cmake --build build -j$(nproc)

# Optional flags:
#   -DBUILD_TESTS=NO   disable tests
#   -DWITH_HTTP=NO     disable HTTP support (removes Boost url/OpenSSL deps)
```

The main binary is output to `build/frost`.

## Running Tests

Always run tests with `FROST_SKIP_HTTP_TEST=1` to avoid real network requests:

```bash
# Run all tests
FROST_SKIP_HTTP_TEST=1 ctest --test-dir build --output-on-failure

# Run a specific test suite (matches by name)
FROST_SKIP_HTTP_TEST=1 ctest --test-dir build -R <test-name>

# Run a specific test binary directly
FROST_SKIP_HTTP_TEST=1 ./build/Frost_Tests/<test-name>
```

Test binaries are placed in `build/Frost_Tests/`. Each test file in `cpp/*/tests/` becomes its own executable via the `make_test()` CMake macro.

Integration tests run `.frst` scripts from `integration-tests/` and are discovered automatically by CTest.

## Architecture

**Pipeline:** source text → parser (Lexy grammar) → AST → evaluator → `Value_Ptr`

### Key modules (`cpp/`)

| Module | Purpose |
|---|---|
| `parser/grammar.hpp` | Complete Lexy-based grammar (49KB) — the source of truth for syntax |
| `ast/` | AST node types; each node implements `.evaluate()` or `.execute()` |
| `value/` | Runtime `Value` type and all operators |
| `symbol-table/` | Lexically-scoped variable bindings (`Symbol_Table` with failover chaining) |
| `functions/` | Closures, lambdas, builtins (26 builtin modules) |
| `prelude/prelude.frst` | Standard library written in Frost (loaded at startup) |
| `import/` | Module import system |

### Value type (`cpp/value/include/frost/value.hpp`)

`Value` is a `shared_ptr`-wrapped `std::variant<Null, Int, Float, Bool, String, Array, Map, Function>`.
- `Null` and `Bool` are singletons (use `Value::null()`, `Value::create(Bool{...})`)
- `Float` rejects NaN and Infinity at construction
- `Map` keys must be non-null primitives (enforced in constructor)
- Use `Value::create(...)` factory methods; direct construction is restricted

### AST evaluation pattern

Each AST node receives a `Symbol_Table&` and returns a `Value_Ptr`. The parser builds the AST; nodes are defined in `cpp/ast/include/frost/ast/`.

### Adding a builtin function

Each builtin module lives in `cpp/functions/builtins/`. Registration is in `cpp/functions/include/frost/builtin.hpp`. A builtin takes a `std::span<const Value_Ptr>` and returns `Value_Ptr`.

## Agent Instructions

The agent must not modify files without explicit instruction.
The primary duties of the agent are writing/updating unit tests and read-only debugging.
When modifying unit tests, always build and run all tests to ensure correctness.
If a valid test fails, the agent should simply inform the user. 

## Running the Interpreter

```bash
./build/frost script.frst          # run a file
./build/frost                      # REPL
./build/frost -e "print(1 + 2)"    # eval expression
./build/frost -d script.frst       # dump AST
```
