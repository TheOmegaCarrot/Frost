# Frost

Frost is a dynamic, immutable scripting language with flexible, expressive syntax.

```frost
# The classic inefficient recursive fibonacci
defn fib(n) -> {
    if n < 2: n
    else: fib(n-1) + fib(n-2)
}

( map range(10) with fib ) @ print()
```

## Building

### Dependencies

The following must be installed on your system:

| Dependency                    | Notes                                                                            |
|-------------------------------|----------------------------------------------------------------------------------|
| C++26 compiler (GCC or Clang) | Minimum GCC 15's stdlib, Clang's stdlib does not support all necessary features. |
| CMake 3.31.6+                 |                                                                                  |
| mold                          | Linker                                                                           |
| Boost                         | `regex`, `json`, `url`, `filesystem` components                                  |

Extensions have their own dependencies which are auto-detected at configure time.
Missing libraries cause the dependent extension (or feature) to be silently skipped by default.
See the [extension documentation](docs/md/stdlib/) for details.

The following dependencies are fetched automatically by CMake:
[lexy](https://github.com/foonathan/lexy),
[{fmt}](https://github.com/fmtlib/fmt),
[replxx](https://github.com/AmokHuginnsson/replxx),
[cppcodec](https://github.com/tplgy/cppcodec),
[Catch2](https://github.com/catchorg/Catch2) (test-only),
[Trompeloeil](https://github.com/rollbear/trompeloeil) (test-only).

Several modules (HTTP, SQLite, etc) are optional extensions that
can be individually enabled or disabled at build time.
See the [complete documentation](docs/md/stdlib/) for build flags and details.

### Compiling

```bash
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTS=NO
cmake --build build -j$(nproc)
```

The interpreter is output to `build/frost`.

## Usage

```bash
frost script.frst         # run a script
frost                     # start the REPL
frost -e "print(1 + 2)"   # evaluate an expression
```

## Documentation

- [Introduction to Frost](docs/md/introduction.md) — language philosophy and a guided tour
- [Language Reference](docs/md/language.md) — complete syntax reference
- [Standard Library](docs/md/stdlib/) — all built-in functions and utilities
