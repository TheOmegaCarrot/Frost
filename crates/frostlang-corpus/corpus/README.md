# The Frost core-language corpus

A large body of standalone Frost scripts, each pinning what the core language
should do with it. Its subject is the domain of the compiler and the VM.
Library behavior (most globals, `std.*`, `ext.*`) belongs to the stdlib's own
tests, and the VM's runtime limits and host-facing configuration (fuel, call
and import depth, cancellation) to the unit tests.

Three kinds of global are in scope all the same:

- The iteration functions `map`, `filter`, `reduce`, and `each`: Frost has no
  loops, so these are how the core language iterates.
- Those bound up with the compiler, which it may inline or optimize: `call` and
  the `is_*` type predicates.
- Those written in Frost itself, in
  [`source.frst`](../../frostlang/src/vm/globals/generated/source.frst): they
  are compiled code like any script.

Any other global may help a script build or observe its result, but is never
its subject.

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
| `lints` | `good`, `runtime-error` | An Array with one entry per lint the script compiles with, in source order: a String, or an Array of Strings, each of which that lint must contain, matched as `rejects` is. Absent, the script must compile with no lints. |
| `inputs` | any | A Map of values by name, each name one a script can bind. The Map is the content of a host component named `inputs`, which the script imports. |
| `known_issue` | any | The ID of the Backlog task tracking a bug that keeps the script from behaving as expected, such as `'FROST-123'`. |
| `regression` | any | `true` if that bug is an accepted regression: the script once behaved as expected. Requires `known_issue`. |
| `bench` | `good`, under `benchmarks/` | Marks the script as a benchmark: `true` to benchmark it with its `inputs`, or `{inputs: {...}}` for larger ones. See [Benchmarks](#benchmarks). |

Any other field is an error.

An import is a runtime call, so an input is unknown to the compiler, and no
optimization can fold or propagate it. A script that must exercise runtime
behavior, rather than what the optimizer computes ahead of time, takes its
values as inputs, importing them in its first statement:

```frost
# An Array pattern destructures a value only known at runtime.
#= {result: [3, 2, 1], inputs: {xs: [1, 2, 3]}}
def {xs} = import('inputs')
match xs {
    [a, b, c] => [c, b, a],
}
```

Every script is compiled on its own, as the CLI compiles a file, so any script
that compiles can be listed with `frost list` to study what the compiler
emits; its import fails only if run outside the corpus.

Error fragments match loosely so that rewording a message does not break the
corpus. Choose the smallest fragment that still tells this error from others.

A lint's entry names its code, and the name it is about when it names one, so
each entry tells its lint from the others:

```frost
# A binding no code reads is still evaluated, so its error still raises.
#= {raises: 'division by zero', lints: [['unused binding', '`unused`']]}
def unused = 1 / 0
2
```

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
Scripts, and each script's option sets, run in parallel; the usual test-harness options (`--test-threads`,
`--list`, `--skip`) apply, and so does `RUST_TEST_THREADS`, which
`--test-threads` overrides.

The corpus runs as part of `cargo test`, under the default optimizations.

## Benchmarks

Tests are about correctness and run anywhere. Benchmarks are about
performance, and their timings mean something only on an otherwise-quiet
machine, so they run through `cargo bench`, never `cargo test`.

A benchmark is a `good` script under `benchmarks/` whose expectation has a
`bench` field. It is still a corpus script: the corpus test pins its
correctness with its own `inputs`. `bench: true` times it with those inputs;
`bench: {inputs: {...}}` supplies larger ones, so that a program small enough
to check quickly still does measurable work when timed. The script is
unchanged either way: it imports whatever inputs it is given.

Before timing anything, the benchmarks check that every script under
`benchmarks/` passes its corpus check, and that every benchmark compiles,
completes with its bench inputs, and gives the same result under each selected
option set. If any fails, nothing is timed. Results under bench inputs are not
compared with an expectation; they need only agree. The check covers every
benchmark whatever the filter, and takes about a second.

### Running

```bash
cargo bench -p frostlang-corpus                                   # every benchmark, default options
cargo bench -p frostlang-corpus -- graphs                         # benchmarks whose ID matches `graphs`
cargo bench -p frostlang-corpus -- /run/                          # only the runs
cargo bench -p frostlang-corpus -- --list                         # list the benchmarks, timing nothing
FROST_BENCH_OPTIONS=each cargo bench -p frostlang-corpus          # default, none, and the defaults without each optimization
FROST_BENCH_OPTIONS='default; constant-fold=false' cargo bench -p frostlang-corpus
```

A benchmark's ID is `<directory>/<file>/<compile|run>/<options>`: the
script's path, whether it times compiling the script or running it, and the
option set's label. The filter after `--` is a regular expression matched
against the ID. Criterion's own options also follow `--`, such as
`--measurement-time 10` for steadier numbers.

By default each benchmark runs under the compiler's default options only.
`FROST_BENCH_OPTIONS` selects others: `each`, or a `;`-separated list of
option sets, each `default` or optimization settings over the defaults, in the
syntax `frost` takes (`none`, `all`, `constant-fold=false`, ...). Two sets
that select the same options are an error.

Each benchmark takes about 8 seconds per option set: a second of warm-up and
three of measurement, for compiling and again for running. A full run under
the default options takes about a quarter of an hour, and under `each` about
ten times that, so filter to what a change touches. To check the harness
works without waiting, cut the times:
`cargo bench -p frostlang-corpus -- graphs --warm-up-time 0.05 --measurement-time 0.2`.

A script that the compiler folds entirely, such as the `*-over-constants`
programs, does no work when run under the default options: its run times only
the fixed cost of starting a run, and its compile times the folding.

### Reading the results

For each benchmark, criterion prints `time: [low estimate high]`: its estimate
of the time of one iteration, with a 95% confidence interval. When there is an
earlier result to compare with, it also prints `change:`, the difference as a
percentage with its own interval and a p-value, and a verdict:
`Performance has regressed.`, `Performance has improved.`,
`Change within noise threshold.`, or `No change in performance detected.`
Every run saves its results as the baseline `base` and compares with the
previous one, so changes appear after a second run without asking.

### Comparing two revisions

1. Make the machine quiet: nothing else running (builds and indexers
   included), on AC power, with the same toolchain for both runs.
2. Check out the first revision and run
   `cargo bench -p frostlang-corpus -- --save-baseline before`.
3. Check out the second, and run
   `cargo bench -p frostlang-corpus -- --baseline before`.

Both runs must use the same filter and the same `FROST_BENCH_OPTIONS`: with
`--baseline`, criterion stops at the first benchmark the baseline lacks. When
the second revision adds or renames benchmarks, use
`--baseline-lenient before`, which compares those it can. To compare two
saved runs without timing again, save the second with
`--save-baseline after` and run `-- --load-baseline after --baseline before`.
When results disagree from one run to the next, the machine was not quiet:
run again.

Baselines live in `criterion/` under the Cargo target directory, which
switching revisions leaves in place and `cargo clean` deletes;
`CRITERION_HOME` moves them. A benchmark is named by its script's path, so a
renamed script starts a new history.
