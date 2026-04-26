# Introduction to Frost

Frost is a small, expression-oriented, functional scripting language.
There are no classes or mutable variables, instead focusing on data flow and functions.

## Basics

```frost
def words = split('the quick brown fox jumps over the lazy dog', ' ')
def long_words = filter words with fn w -> len(w) > 3
def uppercase = map long_words with to_upper
foreach uppercase with print
```

`def` binds a name to a value. Once bound, it cannot be changed.
`map`, `filter`, `reduce` and `foreach` are baked into the language.

`reduce` works similarly:

```frost
reduce [1, 2, 3] with plus
# 6
```

([`plus`](./stdlib/operators.md#plus) is a predefined function which is equivalent to `fn a, b -> a + b`.)

You can also optionally add an `init` clause:

```frost
reduce [1, 2, 3] init: 10 with plus
# 16
```

Note: reducing an empty array without an `init:` clause returns `null`, not zero.
If your data might be empty, always provide `init:`.

```frost
reduce [] with plus          # null
reduce [] init: 0 with plus  # 0
```

## Functions

The earlier example could also be written another way.

```frost
foreach (
    'the quick brown fox jumps over the lazy dog'
        @ split(' ')
        @ select(fn w -> len(w) > 3)
        @ transform(to_upper)
    ) with print
```

Here, we're using the _threading operator_.
`foo @ function(bar)` means exactly the same thing as `function(foo, bar)`.
Above, we're using it to chain a sequence of operations.
The left-hand side is just inserted as the first argument to the function call on the right-hand side.
That threading "pipeline" is effectively rewritten to `transform(select(split('...', ' '), fn w -> len(w) > 3), to_upper)`.
The threaded form is certainly nicer, eh?

Because `map`, `filter`, and `reduce` are keywords, they can't be used in `@` pipelines.
Use their functional equivalents instead: [`transform`](./stdlib/collections.md#transform), [`select`](./stdlib/collections.md#select), and [`fold`](./stdlib/collections.md#fold).

```frost
# These are equivalent:
map items with fn x -> x * 2
items @ transform(fn x -> x * 2)
```

When a threading chain gets long, you'll want to break it across lines.
Outside of parentheses, `@` must start on the same line as its left operand: wrap the chain in `()` to break freely:

```frost
# ok
def result = 'some text here' @ split(' ')
```

```frost
# ok
def result = (
    'the quick brown fox jumps over the lazy dog'
        @ split(' ')
        @ select(fn w -> len(w) > 3)
        @ transform(to_upper)
        @ join(', '))
```

```frost
# syntax error
def result = 'the quick brown fox jumps over the lazy dog'
                @ split(' ')
                @ select(fn w -> len(w) > 3)
                @ transform(to_upper)
                @ join(', ')
```

Functions are also just lambdas bound to a name. The `defn` keyword is shorthand for this pattern.
A multi-line lambda implicitly returns the last expression.

```frost
defn describe(n) -> {
    def word = if n > 0: 'positive' elif n < 0: 'negative' else: 'zero'
    $'${n} is ${word}'
}
# describe(42) == "42 is positive"
```

Format strings are prepended with a `$`, and the format specifiers (`${...}`) must exactly contain names.

Any lambda can be recursive, and the `{}` are optional if the body is just a single expression.
`defn` makes the function name available inside the body:

```frost
defn factorial(n) ->
    if n <= 1: 1
    else: n * factorial(n - 1)
```

## Logic

In Frost, `if` is an expression, and it evaluates to a value; it's not just for control flow.
(Think C's ternary `?:` operator. It's basically the same idea)
Unlike C's ternary, Frost also supports `elif`: `if foo: bar elif baz: beep else: boop`.

`if` switches based on _truthiness_, not just actual booleans.
Every value is truthy, except for `false` and `null`.
Even `0`, `""`, and `[]` are truthy.

For logical operations, `and`, `or`, and `not` are provided.
`and` and `or` do not just evaluate to booleans, instead they short-circuit and return values.
`or` returns the first true operand (or the last operand), and `and` returns the first false operand (or the last operand).
`not`, however, always returns a boolean.

```frost
def config = { ["verbose"]: true, ["max_retries"]: null }

def max_retries = config["max_retries"] or 3
def should_log = config["verbose"] and not config["quiet"]
```

### Do Blocks

Each branch of an `if` takes a single expression.
If you need multiple statements in a branch, use a `do` block:

```frost
defn process(x) -> {
    if x > 0: do {
        def doubled = x * 2
        def label = 'positive'
        $'${label}: ${doubled}'
    }
    else: 'non-positive'
}
```

A `do` block opens a new scope, allows `def` and `defn` statements, and evaluates to its final expression.

Note: bare `{ ... }` is a _map literal_ outside of function bodies or `do` blocks.
Writing `if cond: { def x = ... }` is not valid: use `do { ... }` instead.

## Pattern Matching

Frost has `match` expressions for branching on the shape and contents of a value.

```frost
defn describe(v) -> match v {
    null               => 'nothing',
    n is Int if: n > 0 => 'positive int',
    s is String        => $'string: ${s}',
    [first, ...rest]   => $'array starting with ${first}',
    {name}             => $'has name: ${name}',
    _                  => 'something else'
}
```

Arms are tried top-to-bottom; the first match wins.
`_` is the catch-all.
Bindings like `n`, `s`, `first`, and `name` are scoped to their arm.
Guards (`if:`) add extra conditions after a pattern matches.

Match arms take a single expression, just like `if` branches.
Use `do` blocks when you need multiple statements:

```frost
defn describe(v) -> match v {
    n is Int if: n > 0 => do {
        def doubled = n * 2
        $'positive int, doubled: ${doubled}'
    },
    s is String => $'string: ${s}',
    _           => 'something else'
}
```

See the [language reference](./language.md#match-expressions) for the full details.

## Maps

The previous example also demonstrated maps.
Maps are key-value pairs; arrays, maps, functions, and null may _not_ be map keys, but any value may be a map value.
Accessing a missing key in a map gives you `null`.
There is also a shorter syntax for map keys which are strings following variable naming rules:

```frost
def config = { verbose: true }

def max_retries = config.max_retries or 3
def should_log = config.verbose and not config.quiet
```

This is just normal map accessing, and there's no extra meaning. It's just a bit of a syntactic nicety.
This makes deeply nested maps much nicer to work with, such as you might get as a result of parsing large JSON objects.

`reduce`, `map`, `filter`, and `foreach` are valid to use for maps, but they're not used as often.
See the language reference documentation for a complete explanation.

The infix `+` operator is overloaded for maps, and applies a merge operation.
Note that this builtin operator does not recurse into nested maps.

```frost
{ foo: 42, bar: 10 } + { bar: 81, beep: 256 }
# { foo: 42, bar: 81, beep: 256 }
```

You can also destructure a map in a `def`.

```frost
def person = { name: 'Alice', age: 30, city: 'Portland' }
def { name: n, age: a } = person
# n = "Alice", a = 30
```

You do not need to provide a binding for every key, but every key you name must be present in the map.

## Arrays

We kinda already saw arrays in the first example: a lot of the intermediate values were arrays, but array syntax itself is pretty straightforward:

```frost
def primes = [2, 3, 5, 7, 11]
primes[0] == 2
primes[3] == 7
primes[-1] == 11
len(primes) == 5
primes[1000] == null
```

Similarly to maps, the `+` operator is defined for arrays, and performs concatenation.

```frost
[1, 2, 3] + [4, 5, 6]
# [1, 2, 3, 4, 5, 6]
```

You can also destructure arrays.

```frost
def [a, b, c, ...rest] = primes
# a = 2, b = 3, c = 5, rest = [7, 11]
```

Arrays must destructure into exactly the right number of elements, but you can also collect the rest of the elements into an array of remaining elements.

## Higher-order Functions

Functions can also be variadic.

```frost
def tag = fn label, ...items ->
    map items with fn item -> $'[${label}] ${item}'

# tag('good', 'cheese', 'cats') == [ "[good] cheese", "[good] cats" ]
```

In a variadic function, extra arguments are collected into an array.
When writing higher-order functions, this meshes well with two important functions: [`call`](./stdlib/functions.md#call) and [`try_call`](./stdlib/functions.md#try_call).

`call` takes a function and an array of arguments, and calls that function with those arguments.

```frost
call(plus, [3, 5]) # 8
```

This results in some nice definitions of higher-order functions.
Internally, the function [`curry`](./stdlib/functions.md#curry), which partially applies a function, is defined as:

```frost
def curry = fn f, ...outer -> fn ...inner -> call(f, outer + inner)
```

[`error`](./stdlib/functions.md#error) produces a recoverable error with a message.
[`try_call`](./stdlib/functions.md#try_call) can be used to catch those errors (and any other runtime error).
It takes a function and an array of arguments; if the call succeeds, it returns `{ ok: true, value: result }`, and if it fails, `{ ok: false, error: message }`.

```frost
# Frost catches divisions by 0, but this is illustrative
defn safe_divide(a, b) ->
    if b == 0: error('division by zero')
    else: a / b

try_call(safe_divide, [10, 0])
# { ok: false, error: "division by zero" }

try_call(safe_divide, [10, 2])
# { ok: true, value: 5 }
```

## Modules

Frost has a set of built-in modules accessed via `import`:

```frost
def fs = import('std.fs')
def json = import('std.json')
def io = import('std.io')

fs.exists('data.json')
json.decode(io.open_read('data.json').read_rest())
```

Modules under `std` are always available; modules under `ext` are optional and can be disabled at build time.

You can also split your own code across files.
Any file-scope `def` or `defn` can be `export`-ed:

```frost
# examples/strutils.frst
export defn shout(s)   -> to_upper(s) + '!'
export defn whisper(s) -> to_lower(s) + '...'
```

This can then be `import`-ed:
```frost
# main.frst
def str = import('examples.strutils')
print(str.shout('hello')) # HELLO!
```

Global variables do _not_ leak into the script that calls `import`, and file-based `import` returns a map of all `export`-ed definitions.

That's most of the language! The core of Frost is intentionally pretty small.
For more exact details on exactly how the language works, you can read the [language reference](./language.md).
Frost also provides quite a bit in its [standard library](./stdlib), from math and regex to JSON and HTTP.
