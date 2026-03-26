# Language Reference

This document is a full explanation of the core Frost language, starting from the basics and building up to the more complex elements.
For most readers, this is not the place to start learning the language; for that, see the [introduction](./introduction.md).

<!-- toc -->

- [Lexical Elements](#lexical-elements)
  * [Comments](#comments)
  * [Whitespace](#whitespace)
  * [Identifiers](#identifiers)
- [Types and Literals](#types-and-literals)
  * [Null](#null)
  * [Int](#int)
  * [Float](#float)
  * [Bool](#bool)
  * [String](#string)
  * [Array](#array)
  * [Map](#map)
  * [Functions](#functions)
- [Expressions](#expressions)
  * [Operators](#operators)
    + [Call Threading](#call-threading)
    + [Logical Operators](#logical-operators)
    + [Logical Comparisons](#logical-comparisons)
    + [Arithmetic Operators](#arithmetic-operators)
  * [If Expressions](#if-expressions)
  * [Do](#do)
  * [Iterative Expressions](#iterative-expressions)
    + [Map](#map-1)
    + [Filter](#filter)
    + [Reduce](#reduce)
    + [Foreach](#foreach)
- [Statements](#statements)
  * [Definitions](#definitions)
    + [Array Destructuring](#array-destructuring)
    + [Map Destructuring](#map-destructuring)
- [Modules](#modules)
  * [Export](#export)
  * [Import](#import)
    + [Built-in modules](#built-in-modules)
    + [File-based modules](#file-based-modules)
  * [The `imported` variable](#the-imported-variable)
  * [The `args` variable](#the-args-variable)

<!-- tocstop -->

## Lexical Elements

### Comments

Comments begin with `#` and continue to the end of the line.
There are no multi-line comments.

### Whitespace

Space characters are not significant in Frost; indentation is also not significant.
Line breaks are used to separate statements and expressions.

Semicolons may be used in place of line breaks to separate statements on the same line.

```frost
def a = 1; def b = 2; print(a + b)
```

Inside paired delimiters (`()`, `[]`, and `{}`), line breaks are ignored, since the closing delimiter is unambiguous.
Function calls, indexing, dot map accesses, and the threading operator must begin on the same line as the left operand, unless enclosed in `()`.

```frost
# two expressions
foo
(bar)

# function call
foo(bar)

# function call
( foo
    (bar) )
```

### Identifiers

Frost's identifier rules mirror those of C, and thus should be familiar to most programmers.
Reserved words may not be used as identifiers.
Below is a list of reserved words.

- `and`
- `def`
- `defn`
- `do`
- `elif`
- `else`
- `export`
- `false`
- `filter`
- `fn`
- `foreach`
- `if`
- `init`
- `map`
- `not`
- `null`
- `or`
- `reduce`
- `true`
- `with`


## Types and Literals

There are only 8 types: `Null`, `Int`, `Float`, `Bool`, `String`, `Array`, `Map`, and `Function`.
The types can be grouped into type categories: `Numeric` types are `Int` and `Float`.
`Primitive` types are `Numeric`, `Null`, `Bool`, and `String`.
`Structured` types are `Array` and `Map`.

### Null

The keyword `null` is used to refer to the `Null` value.

```frost
def null_value = null
```

### Int

An `Int` literal is simply a sequence of digits.
`Int`s are all stored as signed 64-bit integers.

```frost
def an_int = 42
```

### Float

A `Float` literal must take the form of a sequence of digits, a `.`, followed by another sequence of digits.
`Float`s are all stored as 64-bit double-precision IEEE-754 floating-point numbers.
A `Float` may not be NaN or infinity.
Digits before and after the decimal point may not be omitted.

```frost
def a_float = 3.14 # ok

def bad_1 = .5 # not permitted, use 0.5
def bad_2 = 3. # not permitted, use 3.0
```

### Bool

A `Bool` is just `true` or `false`.

```frost
def yes = true
def no  = false
```

### String

`String` literals may take a few forms: they may use single-quotes or double-quotes.
There is no semantic difference between the two forms.
All strings are binary-safe, and represent arbitrary sequences of bytes.
String literals may also be raw string literals, which do not process escape sequences.
Multi-line strings are not supported, this includes raw string literals.
Raw string literals begin with `R'(` or `R"(` and end with `')` or `")`. The opening and closing quotes must match.

```frost
def foo = 'foo'
def bar = "bar"
def raw = R'(hello\n)' # literal 'hello\n', where `\n` is not replaced with a newline
```

The supported escape sequences are:

- `\n`: newline
- `\t`: tab
- `\r`: carriage return
- `\\`: literal backslash
- `\"`: double quote (only in `"`-style string)
- `\'`: single quote (only in `'`-style string)
- `\0`: null byte

Format strings are prepended with a `$`, and may be `'`-style or `"`-style strings.
Raw string literals may not be used as format strings.
Format strings are evaluated immediately.
Each `${}` replacement specifier must contain exactly an identifier which is in-scope.
Arbitrary expressions are not supported.
Normal escape sequences are processed before replacement of `${}` specifiers, thus a `${}` specifier may be escaped with `\\`.

```frost
def fmt = $'${an_int} is 42' # 42 is 42
def escaped = $'${an_int} vs \\${an_int} vs \\\\${an_int}' # R"(42 vs ${an_int} vs \42)"
```

### Array

An `Array` is a heterogenous sequence of values.
Any value may appear in an array.
Arrays may be empty.
An `Array` literal is a `,`-separated list of arbitrary expressions enclosed in `[]` brackets.
Trailing commas are permitted, and newlines may appear within the array literal.

```frost
def an_array = [
    'foo',
    40 + 2,
    [],
] # [ 'foo', 42, [] ]
```

Arrays may be indexed directly using familiar postfix `[]` syntax.
Arrays use zero-based indexing, and also support reverse indexing, which should be familiar to Python developers.
Out-of-bounds indexes evaluate to `null`.

```frost
an_array[1] # 42
an_array[-1] # []
an_array[5000] # null
```


### Map

A `Map` is a key-value structure.
`Null`, `Array`, `Map`, and `Function` may not be map keys, but any value may be a map value.
Maps may be empty.
A `Map` literal consists of `,`-separated key-value pairs enclosed in `{}` braces.
Trailing commas are permitted, and newlines may appear within the map literal.

A key-value pair is an expression enclosed in `[]` brackets (which is the key), followed by a `:`, followed by an expression which is used as the corresponding value.
```frost
def a_map = {
    ['foo']: 42,
    ['bar']: 10,
    [40 + 2]: '42',
}
```

Alternately, if a key is a string which conforms to identifier rules, the brackets and quotes may be omitted.
The following example is identical to the previous:

```frost
def a_map = {
    foo: 42,
    bar: 10,
    [42]: '42',
}
```

Just like with identifiers, reserved words may not be used in this way:

```frost
{ ['if']: true } # ok

{ if: true } # syntax error
```

To access a value in a map, you may use indexing similar to arrays.
If a key does not exist, `null` will be returned.

```frost
a_map['foo'] # 42
a_map['missing'] # null
```

Similar to map literals, if a key is a string which conforms to identifier rules, dot map access may be used:

```frost
a_map.foo # 42
a_map.missing # null
```

### Functions

All function definitions are lambda expressions.
To bind a function to a name, simply use the same `def` syntax with a lambda on the right-hand side of the `=`.

A function definition consists of the keyword `fn`, followed by an optional list of arguments, followed by `->`, followed by a body.
The list of arguments may optionally be enclosed in parentheses.
The body is enclosed in `{}` braces, and may consist of multiple statements.
If the body consists of only a single expression, then the enclosing `{}` may be omitted.
This is even true when the single expression appears on the following line.

When a function is called, the final expression is used as the return value.
_All_ functions return exactly one value. It is typical for side-effect functions (such as `print`) to return `null`.

All of the following function definitions are identical in meaning:
```frost
def f1 = fn ( a, b ) -> { a + ( b * 2 ) }

def f2 = fn (a, b) -> a + ( b * 2 )

def f3 = fn a, b -> { a + ( b * 2 ) }

def f4 = fn a, b -> a + ( b * 2 )

def f5 = fn a, b ->
    a + ( b * 2 ) # indentation is not required

f5(2, 4) # 10
```

Map literals can appear directly as a function body, with one exception: when the _first_ entry uses a computed key (`[...]`), the parser cannot distinguish it from a block. Wrap the map in parentheses, or put an identifier-keyed entry first.

```frost
def f1 = fn -> { foo: 42 }           # ok — identifier first key
def f2 = fn -> {}                    # ok — empty map
def f3 = fn -> { [k]: 42 }           # syntax error — computed key is first
def f4 = fn -> ({ [k]: 42 })         # ok — parenthesised
def f5 = fn -> { foo: 1, [k]: 42 }   # ok — identifier key is first
```

A function may contain local `def` and `defn` statements, but neither may end a function body.
The body of a function may not `export` any definitions.

```frost
def f5 = fn a, b -> {
    def c = b * 2
    a + c
} # ok

def f6 = fn a, b -> {
    def c = a * b
} # syntax error

def f7 = fn a, b -> {
    export def c = a * b
    c
} # syntax error
```

A function may _capture_ definitions which appear prior to its own definition.
A function argument may be captured.

```frost
def make_adder = fn a -> {
    fn b -> a + b # a is captured
}

make_adder(5)(3) # 8
```

`{}` braces can also be omitted when returning a new function.
The following is equivalent:

```frost
def also_make_adder = fn a -> fn b -> a + b
```

Functions may also be variadic. A variadic parameter is preceded by `...`, and must appear as the final parameter.
Any extra arguments will be collected into an array which is the variadic argument.
The variadic argument array may be empty.

```frost
def add_things = fn i, ...rest -> reduce rest with plus init: i

add_things(1, 2, 3, 4) # 10 (rest was [2, 3, 4])
```

For a function to be able to recurse, it must capture itself.
For a lambda to capture itself, the following syntax can be used:

```frost
map [1, 2, 3, 4] with fn fib(n) -> if n <= 1: n else: fib(n-1) + fib(n-2)
```

Here, the closure's own name appears before its parameter list.
Note that parentheses are required in this form.

For a function being bound to a definition, a shortened form is available.
The following two snippets are identical in meaning:

```frost
def fib = fn fib(n) -> {
    if n <= 1: n
    else: fib(n-1) + fib(n-2)
}
```

```frost
defn fib(n) -> {
    if n <= 1: n
    else: fib(n-1) + fib(n-2)
}
```

## Expressions

### Operators

Frost provides the typical set of arithmetic, comparison, logical, and postfix operators.
The table below lists all operators from lowest precedence (loosest binding) to highest (tightest binding).

| Precedence | Operators                  | Associativity  |
|------------|----------------------------|----------------|
| 1 (lowest) | `or`                       | left           |
| 2          | `and`                      | left           |
| 3          | `==` `!=`                  | non-chainable  |
| 4          | `<` `<=` `>` `>=`          | non-chainable  |
| 5          | `+` `-`                    | left           |
| 6          | `*` `/` `%`                | left           |
| 7          | `-` (unary) `not`          | prefix         |
| 8 (highest)| `()` `[]` `.` `@`          | postfix (left) |

Equality and comparison operators are _non-chainable_; expressions like `1 < 2 < 3` or `a == b == c` are syntax errors.

```frost
# These are all true:
1 + 2 * 3 == 1 + (2 * 3)
not true or false == (not true) or false
true and false or true == (true and false) or true
-2 + 3 == (-2) + 3

# Left-associative operators group left to right:
10 - 3 - 2 == (10 - 3) - 2
12 / 6 / 2 == (12 / 6) / 2

# Prefix operators apply to the innermost operand:
not not true == not (not true)

# Postfix operators bind tighter than prefix:
-[1, 2, 3][0] == -(([1, 2, 3])[0])

# Postfix operators chain left to right:
def f = fn -> fn x -> x * 2
f()(3) == (f())(3)
```

#### Call Threading

One notable operator which would be unfamiliar is the _threading operator_ (`@`).
Some languages with a similar feature may call this a _pipeline operator_.
This is similar to `->` in Fennel or `|>` in Elixir.

In Frost, `a @ f(b)` is exactly equivalent to `f(a, b)`. (This even results in the same AST!)
This also allows for postfix chaining, such that `a @ f(b) @ g(c)` is identical to `g(f(a, b), c)`.

#### Logical Operators

The behavior of `and`, `or` and `not` should be familiar to users of Python, Lua, or many other languages.
`or` will return its first "truthy" argument, or the final argument if all are "falsy".
`and` will return its first "falsy" argument, or the final argument if all are "truthy".
`not` will always return a boolean which is the inverse of the "truthiness" of its operand.

Evaluation of `and` and `or` short-circuits.

```frost
42 or assert(false) # 42, and assert is not evaluated
false and assert(false) # false, and assert is not evaluated
```

All values are truthy, except for an actual `false` and `null`.

#### Logical Comparisons

Logical comparisons work as one would expect.
The ordering operators (`<`, `>`, etc) compare numeric types as expected, permitting mixing of Int and Float.
(`3 < 3.14` is well-defined.)

The ordering operators also compare String values lexicographically.
(`'foo' > 'bar'` is `true`.)

Equality comparisons (`==` or `!=`) will _never_ cause a type error.

Primitive types, arrays, and maps are compared by value. Functions are compared by identity.

```frost
def a = 42
def b = 42
a == b # true

def arr1 = [1, 2, 3]
def arr2 = [1, 2, 3]
arr1 == arr2 # true
```

#### Arithmetic Operators

`+`, `-`, `*`, `/`, and `%` all work as one would expect.
When mixing `Int` and `Float`, operands are implicitly converted to `Float`.
`%` only supports `Int` operands.
`/` will perform truncating integer division when both operands are `Int`.

`+` has some additional functionality when used with `String`, `Array`, and `Map` operands.
`+` will perform `String` concatenation, `Array` concatenation, and `Map` merge operations.
Note that the `Map` merge is _not_ recursive.

```frost
[1, 2, 3] + [4, 5, 6] # [1, 2, 3, 4, 5, 6]
'foo' + 'bar' # 'foobar'
{ foo: 42, bar: 10 } + { bar: 1024, baz: true } # { foo: 42, bar: 1024, baz: true }
{ foo: { bar: 42, baz: 10 } } + { foo: { bar: 1024 } } # { foo: { bar: 1024 } }
```

### If Expressions

`if` is an expression, and evaluates to a value.
This is similar to the ternary operator (`? :`) in C or C++.

For example:

```frost
def x = if condition(): 42
        elif something_else(): 81
        else: 10
```

Note that the line breaks and indentation are not necessary.
It would be syntactically equivalent to write it all on one line (though it looks pretty cluttered), or with different indentation.

```frost
# equivalent to x
def x2 = if condition(): 42 elif something_else(): 81 else: 10

# also equivalent
def x3 = if condition(): 42
            elif something_else(): 81
    else: 10
```

As one would expect from an `if`, the branch not taken is not evaluated.

```frost
if false: assert(false) # assert is not evaluated
```

If an `if` does not have an `else` clause, it implicitly has an `else: null`.

Each branch of an `if` must be a single expression.

```frost
if foo: { # syntax error
    # multiple statements
}
```

If a multi-statement branch is desired, then you can use a do-block, which is described in the next section.

```frost
if foo: do {
    def x = 10
    x 
}
else: 42
```

### Do

A `do` block is a single expression that begins a new scope, allows `def` and `defn` statements within, and evaluates to its final expression.
This was developed in response to a growing pattern of immediately-invoked functions (`fn -> { ... }()`) being used to scope local definitions.
A `do` block is nearly a drop-in replacement for this ugly but effective pattern, and incurs less runtime overhead.

Any `def` or `defn` statements are entirely scoped to the block itself:

```frost
do {
    def x = 5
    x
}
x # error: Symbol x is not defined
```

A `do` block evaluates to its final expression:

```frost
assert(do { 
    def x = 5  
    x
} == 5)
```

Similarly to block-style functions, a `do` block may not end with a `def` or `defn` statement:

```frost
do {
    def x = 5 # Error: A do block must end in an expression
}
```

### Iterative Expressions

There are 4 iterative expressions built into the language itself: `map`, `filter`, `reduce` and `foreach`.
Each is defined for `Array` and `Map` inputs.

#### Map

A map expression takes the form:

```frost
map an_array with unary_operation
map a_map with binary_operation
```

The `Array` form of a map expression calls a function with each element, in order, and creates a new array with the results of those function calls.

```frost
map [1, 2, 3] with fn x -> x * 2 # [2, 4, 6]
```

The `Map` form of a map expression calls a function with each key-value pair, in an unspecified order, and merges the resulting maps.
The function passed to the `Map` form of a map expression must return a map.

```frost
map {foo: 42, bar: 10} with fn k, v -> ({ [k]: v, [k + k]: v + v })
# { foo: 42, bar: 10, foofoo: 84, barbar: 20 }
```

#### Filter

A filter expression takes the form:

```frost
filter an_array with unary_predicate
filter a_map with binary_predicate
```

In both forms, the function is called with each element or key-value pair.
If the function returns a truthy value, then that element or key-value pair will be preserved in the result.
As with `map`, the iteration order over a `Map` is unspecified.

```frost
filter [1, 2, 3, 4] with fn n -> n % 2 == 0 # [2, 4]
filter {foo: 42, bar: 11} with fn k, v -> v % 2 == 0 # { foo: 42 }
```

#### Reduce

A reduce expression takes one of the following forms:

```frost
reduce an_array with binary_operation
reduce an_array with binary_operation init: value
reduce a_map with ternary_operation init: value
```

In the first form, the function is first called with the first two elements.
The result of this is used as the first argument to the next call, whose second argument is the third element.

In other words, the first argument to the function is an _accumulator_, and each call to the function returns a _new accumulator_ value.

```frost
reduce [1, 2, 3, 4] with fn a, b -> a + b # 10 (the function is called 3 times)
```

Reducing an empty array without an `init:` clause will return `null`.

In the second form, the value after `init:` is used as the initial accumulator value.

```frost
reduce [1, 2, 3, 4] with fn a, b -> { a + b } init: 0 # 10 (the function is called 4 times)
```

Note: in the above example, the `{}` braces in the function body are not mandatory, but they are included here for clarity.

In the third form, a `Map` is being iterated over.
In this form, the function is called with the accumulator, a map key, and the corresponding map value.
The `init:` clause is required for the `Map` form.
Once again, the iteration order over a `Map` is unspecified.

```frost
reduce {foo: 42, bar: 10} with fn acc, k, v -> { acc + v } init: 0 # 52
```

#### Foreach

A foreach expression takes the form:

```frost
foreach an_array with unary_function
foreach a_map with binary_function
```

In the `Array` form, the function is called with each array element in order.
In the `Map` form, the function is called with each key-value pair in an unspecified order.

In both forms, if the function returns a truthy value, iteration stops, and the function is not called again.

A foreach expression always returns `null`.

## Statements

Most of Frost consists of expressions, but there are a few proper statements.
The distinction is that an expression evaluates to a value, whereas a statement does not.

### Definitions

A value may be bound to a name using a `def` statement.

```frost
def a_name = any_expression
```

A name may not be redefined within the same scope:

```frost
def x = 42
def x = 10 # error
```

A function introduces a new scope:

```frost
def x = 42
def f = fn a -> {
    def x = 10
    x + a
}
f(5) # 15
```

#### Array Destructuring

A `def` statement can destructure an `Array`.
The right-hand side may be any expression which evaluates to an `Array`, but these examples only use literals for clarity.

```frost
def [a, b, c] = [1, 2, 3]
# a == 1 and b == 2 and c == 3
```

The number of names on the left must exactly equal the number of elements in the array on the right.

Array destructuring can also bind an unknown number of elements to an array:

```frost
def [a, b, ...rest] = [1, 2, 3, 4]
# a == 1 and b == 2 and rest == [3, 4]
```

In this form, all of the names before the `...` must match an element, and the `...` name may bind an empty array.

```frost
def [a, b, ...rest] = [1, 2]
# a == 1 and b == 2 and len(rest) == 0
```

#### Map Destructuring

A `def` statement can destructure a `Map`.
The right-hand side may be any expression which evaluates to a `Map`, but these examples only use literals for clarity.

```frost
def { foo: bar, beep: boop, no: nada } = { foo: 42, beep: 10, dropped: 128 }
# bar == 42 and boop == 10 and nada == null
```

Every key on the left is looked up in the map on the right.
The "value position" on the left contains a name to be bound to the value at that key.
Keys which are not found in the map are bound to `null`.
This matching does not need to be exhaustive (it is not an error that the value at key `dropped` was not bound to a name).

The keys on the left do not necessarily need to be string keys, though in practice they often are.
The following is also valid:

```frost
def { [42]: foo, [false]: bar } = { [42]: 'wow', [false]: 'neat' }
# foo == 'wow' and bar == 'neat'
```

Oftentimes, a pattern such as the following is desireable:

```frost
def { foo: foo, bar: bar } = # something
```

In this case, a shortened form can be used:

```frost
def { foo, bar } = # something
```

The above two examples are identical in meaning.

## Modules

Frost provides a mechanism for using code across separate files.
One file may use the `import` function to import what is `export`-ed by another file.

### Export

Any `def` or `defn` statement at file scope may be prepended with the `export` keyword.
Any definition marked as `export` will be made available when a script is imported.
This does not change the meaning within that script file, and the name(s) can be used normally.
The rules for an exported definition are the same as described in previous sections.

```frost
export def foo = 42
# foo == 42
# if this file is imported, then foo will be exported
```

`export defn` is the shorthand form for exporting a named function:

```frost
export defn greet(name) -> $'hello, ${name}'
```

It is an error for an `export def` or `export defn` to appear within a function:

```frost
def f = fn -> {
    export def foo = 10 # error
    foo
}
```

### Import

The function `import` takes a module specification and returns a value.
The module specification must be a non-empty `String`.

#### Built-in modules

Frost ships with built-in modules under the `std` and `ext` namespaces.
These are available without any files on disk.

```frost
def fs = import('std.fs')
def http = import('ext.http')
```

A complete list of built-in modules is in the [standard library documentation](./stdlib).

The built-in module registry is a nested `Map`.
`import` walks the `.`-separated path segments through this structure and returns whatever value it reaches.
This means you can import at any level of granularity:

```frost
def std = import('std')           # the entire standard library (a Map of Maps)
def b64 = import('std.encoding.b64')  # a specific sub-map
def encode = import('std.encoding.b64.encode')  # a single function
```

Because `import` returns a `Map`, you can even apply destructuring to it:

```frost
def { fs, io, json } = import('std') # imports fs, io, and json in one line
def { http, sqlite } = import('ext')
```

#### File-based modules

If no built-in module matches, `import` treats the specification as a relative path using `.` as a path separator.
The `.frst` file extension is added automatically.
For example: `'foo.bar.baz'` specifies a relative path `foo/bar/baz.frst`.
The module specification `'foo'` specifies a relative path `foo.frst`.

The result is a `Map` with keys being the exported names, and values being the values bound to those names.

`import` will first check relative to the importing file.
If no file can be found relative to the importing file, the interpreter will then look relative to the current working directory.

It is an error if a module import cannot be resolved.

If no file is found, then a list of paths will be looked up in the environment variable `FROST_MODULE_PATH`.
This environment variable consists of a colon-separated list of paths (relative or absolute).
Each path in the list is tried as a base directory for the module specification.

Below are some examples demonstrating how module paths are resolved.
Note that useful exports have been omitted for brevity in some cases.


```
project/
├── main.frst
└── utils.frst
```

```frost
# utils.frst
export def greet = fn name -> $'hello, ${name}'

# main.frst
def utils = import('utils')
utils.greet('world') # "hello, world"
```

Dots in the module specification correspond to directory separators:

```
project/
├── main.frst
└── lib/
    └── math/
        └── vectors.frst
```

```frost
# main.frst
def vec = import('lib.math.vectors')
```

Because `import` resolves relative to the importing file, not the current working directory, two files can import different modules with the same specification:

```
project/
├── main.frst
├── helpers.frst
└── lib/
    ├── core.frst
    └── helpers.frst
```

```frost
# lib/core.frst
def h = import('helpers') # resolves to lib/helpers.frst, not project/helpers.frst

# main.frst
def h = import('helpers')     # resolves to project/helpers.frst
def core = import('lib.core') # resolves to project/lib/core.frst
```

### The `imported` variable

Every Frost file has access to a predefined `Bool` named `imported`.
It is `false` when the file is run directly, and `true` when it is loaded via `import`.

This allows a file to act as both a reusable module and a runnable script:

```frost
export def greet = fn name -> $'hello, ${name}'

if not imported: print(greet('world'))
```

### The `args` variable

When running a file, `args` is prefined as an `Array` of `String` values containing the command-line arguments passed to the program.
`args[0]` is the path to the main script file, and any subsequent elements are any additional arguments.

```
frost script.frst foo bar   # args == ["script.frst", "foo", "bar"]
```
