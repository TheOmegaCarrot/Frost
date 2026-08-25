# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Project Overview

Frost is a dynamically-typed functional programming language. It is an embeddable scripting
language first, in the mold of Lua: designed to be consumed as a library by a host application.

This repository is the **Rust bytecode implementation**: source text is parsed to an AST,
compiled to bytecode, and executed on a stack VM. It is the top-level project.

The former C++ tree-walking interpreter lives under `frost-cpp/` and is legacy. It serves
only as a behavioral oracle; do not extend it. A bug in it is a nonissue.

Frost is not yet stabilized or published.

## Build & Test

The toolchain is the current stable Rust, unpinned; resolve it live (`rustc --version`)
rather than assuming a version. Edition 2024.

```bash
cargo build                       # debug build
cargo build --release             # release build
cargo test                        # run the whole workspace's tests
cargo test -p frost-parse         # test a single crate
cargo run -p frost-cli -- file.frst   # run a Frost script
cargo doc --no-deps --workspace   # build RustDoc (intra-doc links must resolve clean)
cargo clippy --workspace --all-targets
cargo fmt
```

## Architecture

A workspace of focused crates:

| Crate | Purpose |
|---|---|
| `frost-parse` | Source text to AST: lexer, recursive-descent parser, diagnostics. Grammar only; semantics belong to the compiler. Intentionally lax in what it accepts, deferring many errors to the compiler. |
| `frost-runtime` | The `core` (the `Value` type, its variants, operators, conversions) and the `vm` (bytecode execution, globals, native functions, arity/type params, import, serialization). |
| `frost-cli` | The `frost` binary; runs a `.frst` file. |
| `frost-astviz` | AST visualization; compiles the tree-sitter Frost grammar from `editor/`. |

The compiler (AST to bytecode) is planned as its own crate and is not yet present; some
globals are intentionally stubbed until it lands.

Design documents and working scratch live in `tmp/` (git-ignored) at the repo root.

## Frost Syntax

`crates/frost-parse/tests/` is the authoritative behavior spec for syntax. The following is
a quick guard against common mistakes, not an exhaustive reference.

### Comments

Python-style, ASCII only.

```frost
# a comment to end of line
run_this()
```

### `if` / `elif` / `else`

Colon syntax, not braces or `then`. Each branch is a single expression.

```frost
if x: 1
elif y: 2
else: 3

if condition: print("yes")   # else: null implicitly
```

Braces are not valid branch syntax. For multiple statements, use a `do` block:

```frost
if condition: do {
    def x = 1
    x + 2
}
else: 0
```

### `@` threading operator

`a @ f()` means `f(a)`. `a @ f(x)` means `f(a, x)`: the left value is threaded in as the
first argument. Parens are always required.

```frost
[1, 2, 3] @ transform(fn x -> x * 2)   # transform([1, 2, 3], fn x -> x * 2)
```

### Format strings

`${}` interpolates any expression; `\$` escapes interpolation; a bare `$` not followed by `{`
is literal.

```frost
def name = "world"
$'hello, ${name}'            # "hello, world"
$'result: ${1 + 2}'         # "result: 3"
$'literal: \${name}'        # "literal: ${name}"
```

### Functions

```frost
fn x -> x + 1                    # single-expression lambda
fn x, y -> x + y                 # multiple parameters
fn -> { def x = 1; x + 2 }       # block body (statements separated by newlines or ;)
fn x -> fn y -> x + y            # curried
fn fact(n) -> if n <= 1: 1 else: n * fact(n - 1)   # named lambda (name usable for recursion)
```

`defn name(params) -> body` is sugar for `def name = fn name(params) -> body`:

```frost
defn add(x, y) -> x + y
export defn greet(name) -> $'hello, ${name}'
```

Abbreviated lambdas: `$(expr)` with `$` / `$1` to `$9` placeholders, `$$` for rest args.

```frost
map [1, 2, 3] with $($ * 2)      # [2, 4, 6]
$($1 + $2)                       # fn $1, $2 -> $1 + $2
```

### Truthiness

Only `null` and `false` are falsy. `0`, `""`, `[]`, `{}` are all truthy.

### `and` / `or`

Short-circuit and return the actual operand value, not a coerced Bool.

```frost
null or "default"    # "default"
42 and "yes"         # "yes"
false or 0           # 0
```

### Comparisons

Relational (`<` `<=` `>` `>=`) and equality (`==` `!=`) operators do not chain or mix; wrap
in parens to combine. `3 == 3.0` is False: `==` has no cross-type numeric equality, though
`<` and `>` do compare across Int and Float.

### Match expressions

```frost
match value {
    42               => 'the answer',      # literal value pattern
    n is Int         => $'int: ${n}',      # binding with type constraint
    (some_var)       => 'matched var',     # parenthesized: compares against an existing variable
    [a, b, ...rest]  => a + b,             # array pattern with rest
    {name, age: a}   => a,                 # map pattern (shorthand plus explicit)
    {name} as person => person.age,        # map pattern with whole-map binding
    1 | 2 | 3        => 'low',             # alternatives: first match wins
    _ if: guard      => 'guarded',         # guard with `if:` (note the colon)
    _                => 'catch-all'        # discard binding
}
```

- Arms are comma-separated, even across newlines; a trailing comma is permitted.
- No matching arm is a runtime error, not null.
- A bare identifier is a binding; `(expr)` and literals are value comparisons.
- Map patterns require the key to be present; a missing key is a no-match.
- Alternatives (`|`) work at any nesting level; all branches must bind the same set of names.
- `as` whole-value binding is map-only, not arrays.
- Type constraints: `Null`, `Int`, `Float`, `Bool`, `String`, `Bytes`, `Array`, `Map`,
  `Function`, `Primitive`, `Numeric`, `Structured`, `Flat`, `Nonnull`.

### Map / filter / reduce expressions

Dedicated `with` syntax, not function calls:

```frost
map [1, 2, 3] with fn x -> x * 2
filter [1, 2, 3] with fn x -> x > 1
reduce [1, 2, 3] with fn (acc, x) -> acc + x
```

The pipeline-friendly function forms are `transform`, `select`, and `fold`.

## Principles

### Design philosophy

- Frost is embeddable-first: a library a host embeds, in the spirit of Lua.
- Frost values are immutable, and the compiler and VM are built on that assumption. It is
  what enables structural optimizations (for example, reusing a uniquely-owned buffer in
  place instead of copying). Lean on immutability; do not design around it.
- Keep suggestions Frost-shaped: functional and cohesive with the rest of the language. How
  another language solves something is a case study, not an argument. Different languages
  make different choices because they hold different philosophies, constraints, and
  assumptions; weigh an idea on whether it fits Frost.

### Rust code

- Stable Rust only, against the current toolchain. Recently-stabilized features are fine.
- `unsafe` requires very strong justification.
- Take full advantage of strong types. Make invalid states unrepresentable wherever possible.
- Write idiomatic Rust.
- The Frost core stays dependency-light: pulling in a minimal Frost (runtime plus compiler)
  should be fairly lightweight. Optional extensions may be heavier.
- Prefer self-documenting code: good names and clear algorithms (clarity that assumes Rust
  fluency is still clarity). Where clarity cannot reasonably be achieved without a
  performance cost, add an explanatory comment for the unclear part.

### Testing

- Strongly prefer black-box tests (public API only). Reach for white-box tests only when
  something genuinely prevents a black-box test.
- Black-box tests live in `crates/*/tests/`. When a white-box test is unavoidable, put it in
  its own `#[cfg(test)] mod name;` file, not inline in a large source file.
- Tests are deterministic, reliable, thorough, comprehensive, and borderline paranoid.
- Tests carry internal documentation and give clear assertion failure messages.
- Test code is clear and easy to read; readability outranks test performance.
- Every Frost-facing function has its arity and type-checking surface tested carefully.
- Every bug fix earns a regression test targeting that bug.

### Dependencies

- Inspect every third-party dependency before use.
- Every direct dependency must be actively maintained and well-reputed.

### Prose (comments, error strings, docs)

- Clear, concise, to the point: no more words than needed to make the point clearly.
- No em-dashes, including the `--` ASCII approximation. Use a colon, semicolon, sentence
  break, or a small rephrase.
- ASCII only, unless a non-ASCII character serves a specific purpose.
- Write for the reader: internal comments for people who know or are learning the internals;
  RustDoc for the library user (a Rust developer using Frost's public API); user-facing
  strings for script authors.
- Frost type names are always capitalized (`String`, not `string`).
- Do not repeat documentation or comments. State a fact once, in one home, and reference or
  link to it from elsewhere. Duplication is rot-prone: copies drift out of sync.
- Reference only the current state of the project. A comment about a fixed bug is noise.
- Put a comment next to the code it describes. Never reference line numbers; they change.
- Evaluate every comment for rot. Remove or rephrase rot-prone comments. A comment tied to a
  quantity likely to change is rot-prone (bad: "33 of the 57 functions here are done").
- A deferred task or known gap gets a comment beginning with the exact string `TODO: `.

### RustDoc

- Every public API surface has RustDoc, following the prose rules above.
- Write for the user: they need to use the item correctly, effectively, performantly, and
  safely, not to know how Frost works internally.
- Do not expose internal implementation details. The public API is the stable contract even
  where the implementation behind it may change.
- Document only the item the doc is attached to. To refer to another item, link to it rather
  than re-explaining it in prose.

### Working

- Never edit this `CLAUDE.md` without explicit instruction to do so.
- Everything saved to Claude's memory must be durable and evergreen: unlikely to need revision.

### Pre-publish status (temporary)

- Frost is not yet stabilized or published. A public API change is a non-issue before the
  first publish.
- Not everything a user needs belongs in RustDoc. Content that does not fit cleanly into
  documenting its own item goes in a design document instead.
- A whole-library onboarding document is planned but will be written shortly before the first
  publish. Think of RustDoc as Rust's stdlib API reference, and that future document as "The
  Book".
