# EBNF issues (current)

## Mismatches vs design
- Grammar does not model `def` declarations or the separation between declarations and assignments.
- Function literals require `fn` in the design, but the grammar allows `fn_defn` in expression positions without `def` or statement context.
- `return` is statement-only in the design, but the grammar embeds it as a special case at the end of `long_fn_defn`.
- Newline handling is not aligned with "newline is whitespace or separator"; `sep` is empty-capable and `blank`/`mustblank` do not enforce spacing.
- `into map` is not modeled.
- `map` returning maps (callback returns map merged into accumulator) is not represented.
- Map -> array is disallowed in design but not constrained by grammar.
- `#` comments are not modeled.
- Keywords are not reserved in the grammar.
- Map key rules (`k1: v` vs `[expr]: v`) do not match the updated design doc examples.

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
- Update map/array literals, access, and `map`/`reduce`/`foreach` to match design semantics (including `into map`).
- Add terminals for identifiers, comments, keywords, and literals (`null`, comparisons, `and/or/not`).
