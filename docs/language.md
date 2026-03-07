# Language Reference

This document is a full explanation of the core Frost language, starting from the basics and building up to the more complex elements.
For most readers, this is not the place to start learning the language; for that, see the [introduction](./introduction.md).


## Lexical Elements

### Comments

Comments begin with `#` and continue to the end of the line.
There are no multi-line comments.

### Whitespace

Space characters are not significant in Frost; indentation is also not significant.
Line breaks are used to separate statements/expressions, similarly to semicolons, which are also supported.
Inside paired delimiters ( `()`, `[]`, and `{}` ) line breaks are ignored.
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
Raw strings may _not_ be multi-line.
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
`Null`, `Array`s, `Map`s, and `Function`s may not be map keys, but any value may be a map value.
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

Alternately, if a key is a string which conforms to identifier rules, the `['']` may be omitted.
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

A function definition consists of the keyword `fn`, followed by a list of arguments, followed by `->`, followed by a body.
The list of arguments may optionally be enclosed in parentheses.
The body is enclosed in `{}` braces, and may consist of multiple statements.
If the body consists of only a single expression, then the enclosing `{}` may be omitted.
When a function is called, the final expression is used as the return value.
_All_ functions return exactly one value. It is typical for side-effect functions (such as `print`) to return `null`.

All of the following function definitions are identical in meaning:
```frost
def f1 = fn ( a, b ) -> { a + ( b * 2 ) }
def f2 = fn (a, b) -> a + ( b * 2 )
def f3 = fn a, b -> { a + ( b * 2 ) }
def f4 = fn a, b -> a + ( b * 2 )

f4(2, 4) # 10
```

A function may contain local `def` statements, but a `def` statement may not end a function.
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

`{}` braces are not required when returning a new function.
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

A function may be recursive. Every function implicitly captures itself as `self`.
Note that the name `factorial` is _not_ available within the below function definition, as it is not defined until _after_ the lambda expression.

```frost
def factorial = fn n ->
    if n <= 1: 1
    else: n * self(n - 1)
```

There is a common pitfall when using `self` to recurse.
Because `self` always refers to the directly-enclosing function, you must bind `self` to a new name for an inner function to recurse into an outer function.

```frost
# BUG! self is the map lambda, not recursively_double
def recursively_double = fn arr ->
    map arr with fn item ->
        if is_array(item): self(item)
        else: item * 2

# Fix: bind the outer lambda's self to a new name which can be captured
def recursively_double = fn arr -> {
    def recurse = self
    map arr with fn item ->
        if is_array(item): recurse(item)
        else: item * 2
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

Evaluation of `and` and `or` short-circuit.

```frost
42 or assert(false) # 42, and assert is not evaluated
false and assert(false) # false, and assert is not evaluated
```

All values are truthy, except for an actual Boolean `false` and `null`.

#### Logical Comparisons


