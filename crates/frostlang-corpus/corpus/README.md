# The Frost core-language corpus

A large body of standalone Frost scripts, each pinning what the core language
should do with it. Its subject is the domain of the compiler and the VM;
library behavior (globals, `std.*`, `ext.*`) belongs to the stdlib's own tests.
A script may use a few globals to build or observe its result, but never to
test them.

The corpus states intended behavior, never merely current behavior. A script
that shows a bug keeps the correct expectation and carries a known-issue
marker. See `MAINTENANCE.md` for the rules of keeping it.

## Layout

```
corpus/<semantic area>/.../<outcome>/<name>.frst
```

Directories group scripts by semantic area, nested as deep as helps. The last
directory names the outcome every script in it expects:

| Directory | The script |
|---|---|
| `good/` | compiles, and its run completes |
| `runtime-error/` | compiles, and its run raises an error |
| `compile-error/` | does not compile |

A file in the corpus is either a script (`.frst`) or documentation (`.md`);
anything else fails the run, so a misnamed script cannot silently go unrun.

## A script

```frost
# A binding in a `do` block shadows the outer one only until the block ends.
#= {result: [2, 1]}
def x = 1
def inner = do {
    def x = 2
    x
}
[inner, x]
```

It opens with a brief prose comment saying what it checks and, for a subtle
case, why it is subtle. Then comes its expectation block: the lines beginning
with `#=`, which must be contiguous. With the prefix (and one space after it)
removed, they are a Frost expression evaluating to a Map. A long expectation
spans several lines:

```frost
#= {
#=     result: 42,
#=     exports: {answer: 42},
#=     printed: ['computing'],
#= }
```

Write the expected values as literals, so each states the plain answer rather
than computing it.

## The expectation

The Map has exactly one of `result`, `raises`, and `rejects`, which must agree
with the script's outcome directory.

| Field | Outcome | Meaning |
|---|---|---|
| `result` | `good` | The run's tail value, compared with Frost's `==`. Avoid Function and Opaque results: they cannot be pinned stably. |
| `exports` | `good` | A Map of every export by name. Absent, the script must export nothing. |
| `printed` | `good` | An Array of the text of each `print`, in order. Absent, the script must print nothing. |
| `raises` | `runtime-error` | A String, or an Array of Strings, each of which the error's message must contain, ignoring case. |
| `rejects` | `compile-error` | As `raises`, matched against the diagnostics: their messages, labels, help, and codes, but not the source lines they quote. |
| `known_issue` | any | The ID of the Backlog task tracking a bug that keeps the script from behaving as expected, such as `'FROST-123'`. |
| `regression` | any | `true` if that bug is an accepted regression: the script once behaved as expected. Requires `known_issue`. |

Any other field is an error.

Error fragments match loosely so that rewording a message does not break the
corpus. Choose the smallest fragment that still tells this error from others.

## Verdicts

Each script is checked under the run's compiler options, which must all agree
on its outcome (see Running). The script then:

- **passes** if it behaves as expected;
- **fails** if it does not;
- **panics**, a failure whatever it expects: no script may panic or trip a
  debug assertion;
- is a **known issue**, which does not fail the run, if it carries
  `known_issue` and fails or panics;
- is **resolved**, which fails the run, if it carries `known_issue` but now
  behaves as expected: remove the marker;
- is **malformed**, which fails the run, if it is outside an outcome
  directory, or its expectation is missing, unreadable, or disagrees with its
  directory. A `known_issue` marker never excuses this.

After the run, a summary counts the verdicts for each outcome.

## Running

```bash
cargo test -p frostlang-corpus --test corpus                    # default optimizations
FROST_CORPUS_PERMUTATIONS=1 cargo test -p frostlang-corpus --test corpus
EXHAUSTIVE_COMPILER_TESTS=1 cargo test -p frostlang-corpus --test corpus
cargo test -p frostlang-corpus --test corpus -- match/          # scripts whose name contains `match/`
cargo test -p frostlang-corpus --test corpus -- --exact match/good/alternatives
```

By default, each script runs under the compiler's default options only. With
`FROST_CORPUS_PERMUTATIONS` set, it runs under no optimization, every
optimization, each one alone, and all but each one. With
`EXHAUSTIVE_COMPILER_TESTS` set, it runs under every permutation. Each
variable may be set to any value. Optimization must never change what a
program does, so under each mode the options must all agree.

A script's test name is its path under `corpus/`, without the extension.
Scripts run in parallel; the usual test-harness options (`--test-threads`,
`--list`, `--skip`) apply.

The corpus runs as part of `cargo test`, under the default optimizations.
