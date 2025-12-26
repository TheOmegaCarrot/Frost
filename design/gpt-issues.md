# Grammar issues

## EBNF correctness
- Missing semicolons after `fn_defn`, `digit`, `reduce_expr`, `map_expr`, and `foreach_expr`.
- `long_fn_defn` is missing a comma in `blank '}'` (should be `blank, '}'`).
- `mustblank = { ' ' | '\t' | '\n' }` allows empty; it does not enforce required whitespace.
- `blank = [ mustblank ]` is also always empty-capable, so it never enforces whitespace.
- `sep = { '\n' | ';' }` allows empty, so separators are effectively optional everywhere.
- `int_literal = { digit }` can be empty; `float_literal` can match just `.`.

## Lists and argument forms
- `array_construction`, `map_construction`, and `fn_call` only accept elements that are followed by a comma, so single-element or last-element forms (e.g., `[1]`, `%{ k1: 1 }`, `f(a,b)`) do not parse unless they end with a trailing comma.

## Expression ambiguity and recursion
- `binary_infix_expr = expr op expr` combined with `expr -> binary_infix_expr` is left-recursive and ambiguous; there is no precedence or associativity (e.g., `a + b * c`).
- `unary_prefix_expr` is mixed into `expr` without precedence tiers, compounding ambiguity.

## Control forms and separators
- `if_expr` else-branch syntax is wrong: it requires `else <expr> : <expr>` but the design uses `else: <expr>`.
- `if_expr` and `reduce_expr` include `sep` inside the expression, which will force double separators when used inside `{ (expr | decl), sep }`.

## Declarations vs expressions
- `decl` already ends with `sep`, but function bodies use `(expr | decl), sep`, so a declaration inside a function body would require two separators.
- If function bodies are only sequences of expressions, assignment (e.g., `a = 2`) needs to be an expression form; currently it is only modeled as `decl`.

## Access/call chaining
- `array_access` and `long_map_access` are identical, so `a[1]` is ambiguous between them.
- Access/call receivers are limited to `id` or parenthesized `expr`, preventing natural chaining like `a.b.c` or `foo()[0]` without extra parentheses.

## Missing terminals / mismatches with the design
- `id` and `chars` are undefined.
- No comment syntax for `#`.
- The design uses `null`, comparison ops (`<=`, `==`), logical ops (`and`, `or`, `not`), and string/number map keys like `"k2":` and `4:`; none are in the grammar.
- `fn { ... }` sugar exists in the design but not in the grammar.
