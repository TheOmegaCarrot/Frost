# Frost

Frost is a dynamic, immutable scripting language with flexible, expressive syntax.

```frost
# The classic inefficient recursive fibonacci
def fib = fn n -> {
    if n < 2: n
    else: self(n-1) + self(n-2)
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
| Boost                         | `regex`, `json`, `headers` components                                            |
| Boost `url` + OpenSSL         | Optional; required for HTTP support                                              |

The following dependencies are fetched automatically by CMake:
[lexy](https://github.com/foonathan/lexy),
[{fmt}](https://github.com/fmtlib/fmt),
[replxx](https://github.com/AmokHuginnsson/replxx),
[Catch2](https://github.com/catchorg/Catch2) (test-only).

The following dependencies are vendored in `external/`:
[cppcodec](https://github.com/tplgy/cppcodec),
[Lyra](https://github.com/bfgroup/Lyra),
[Trompeloeil](https://github.com/rollbear/trompeloeil) (test-only).

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
frost -e "print(1 + 2)"  # evaluate an expression
```

## Documentation

NOTE: Documentation is heavily work-in-progress!

- [Introduction to Frost](docs/introduction.md) — language philosophy and a guided tour
- [Language Reference](docs/language.md) — complete syntax reference
- [Standard Library](docs/stdlib/) — all built-in functions and utilities
