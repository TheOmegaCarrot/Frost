# EBNF issues (current)

## Mismatches vs design
- Grammar does not model `def` declarations or the separation between declarations and assignments (assignment is statement-only and requires an existing binding).
- `return` is statement-only in the design, but the grammar only permits it as a final optional clause at the end of `long_fn_defn`.
- Newline handling is not aligned with "newline is whitespace or separator"; `sep` is empty-capable and `blank`/`mustblank` do not enforce spacing.
- `#` comments are not modeled.
- Keywords are not reserved in the grammar.

## EBNF correctness / structural issues
- Missing semicolons after `fn_defn`, `digit`, `reduce_expr`, `map_expr`, and `foreach_expr`.
- `long_fn_defn` is missing a comma in `blank '}'` (should be `blank, '}'`).
- `sep = { '\n' | ';' }` allows empty, so separators are effectively optional everywhere.
- `mustblank = { ' ' | '\t' | '\n' }` allows empty; it does not enforce required whitespace.
- `blank = [ mustblank ]` is also always empty-capable, so it never enforces whitespace.
- `int_literal = { digit }` can be empty; `float_literal` can match just `.`.
- `array_construction`, `map_construction`, and `fn_call` only accept elements followed by a comma; last-element forms without trailing comma fail.
- `binary_infix_expr = expr op expr` combined with `expr -> binary_infix_expr` is left-recursive and ambiguous.
- `unary_prefix_expr` is mixed into `expr` without precedence tiers.
- `if_expr` syntax is wrong: it requires `else <expr> : <expr>` but the design uses `else: <expr>`.
- `if_expr` and `reduce_expr` include `sep` inside the expression, which will force double separators when used inside `{ (expr | decl), sep }`.
- `array_access` and `long_map_access` are identical, so `a[1]` is ambiguous between them.
- Access/call receivers are limited to `id` or parenthesized `expr`, preventing chaining like `a.b.c` or `foo()[0]`.
- `id` and `chars` are undefined.

## Todo items (ordered by importance)
- Rewrite separators/whitespace to match the newline policy and make `sep` non-empty.
- Add `def` declarations and distinguish declaration vs assignment forms.
- Rebuild expression grammar with precedence tiers and postfix chaining.
- Fix control forms (`if`/`elif`/`else`, `return` as statement) and remove embedded `sep` from expressions.
- Add terminals for identifiers, comments, keywords, and literals (`null`, comparisons, `and/or/not`).
