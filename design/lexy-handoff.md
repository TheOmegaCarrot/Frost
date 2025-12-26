# Lexy knowledge handoff (from this conversation)

## Scope and certainty
- I did not read Lexy documentation or source code in this conversation.
- Everything below is either inferred from general prior knowledge or derived from how the user framed their grammar needs.
- Treat all Lexy‑specific details as provisional until verified against official docs.

## What I believe about Lexy (high level)
- Lexy is a C++ parsing library with a DSL for defining grammars and tokenization rules.
- Grammars are typically expressed as C++ types/structs with a `rule` (or similar) member that composes DSL primitives.
- The DSL generally supports:
  - sequencing, branching/choice, repetition, optional parts
  - token/lexeme boundaries and custom whitespace handling
  - explicit literal/keyword tokens
  - operator precedence via layered rules or expression helpers
  - lookahead or conditional parsing in some form
- Lexy distinguishes lexing/tokenization concerns (whitespace, comments, tokens) from parsing rules, though the boundary is flexible.

## Practical implications inferred in this conversation
These are not Lexy facts, but constraints we discussed when thinking about writing a Lexy grammar for the language in `design.frst`:

- **Newline handling**: The language wants newlines to sometimes terminate expressions but also allow continuation. This implies a custom whitespace rule that may treat newline specially (e.g., exclude it from “normal” whitespace, then explicitly consume it in separators).
- **Keyword vs identifier**: Keywords like `if`, `reduce`, `map`, `foreach`, `return`, `true/false/null` are not valid identifiers. That implies a lexer rule that reserves these tokens, and an identifier rule that rejects them (or a keyword table checked after lexing).
- **Function literal vs parenthesized expression**: `(x) -> { ... }` and `(expr)` share a prefix. A Lexy grammar would likely require lookahead (e.g., `)->`) or a prioritized branch to disambiguate.
- **Expression sequencing**: Function bodies are sequences of expressions with optional separators (newline/semicolon). That suggests a grammar rule like `expr_list = expr (sep expr)* [sep]`, where `sep` can include newline and `;`.
- **Operator precedence**: The EBNF in `grammer.ebnf` used `expr -> expr op expr`, which is ambiguous and left‑recursive. A Lexy grammar should use precedence tiers or a dedicated expression helper to avoid ambiguity.
- **Postfix chaining**: Calls, indexing, and dot access (`a.b`, `a[0]`, `f(x)(y)`) should chain. The grammar should model a `primary` followed by zero or more postfix operators.
- **Map/array literals**: List/kv‑pair rules must allow a final element without a trailing comma, plus optional trailing comma. That generally means `list = [ item (',' item)* [','] ]`.
- **If expression**: `if <cond>: <expr> elif ... else: <expr>` is an expression form and should be parsed in a dedicated non‑operator production (so `else` doesn’t get eaten as an identifier or part of a lower‑precedence expr).
- **reduce/map/foreach expressions**: These are keyword‑led expressions with fixed structure (`reduce <expr> with <expr> [init: <expr>]`), best modeled as their own productions rather than as infix operators.

## Known grammar issues (from the EBNF)
This is the summary that was written to `../frost/design/gpt-issues.md`:
- Missing semicolons and a missing comma in EBNF.
- `mustblank`, `blank`, and `sep` all accept empty, so they don’t enforce spacing/termination.
- Literals can be empty (`int_literal` and `float_literal` rules).
- List/argument rules only accept comma‑terminated items (no last element without comma).
- Expression rules are ambiguous and left‑recursive.
- `if_expr` else branch syntax is wrong; `if_expr`/`reduce_expr` embed separators that conflict with surrounding lists.
- Declarations end with separators but are also followed by separators in function bodies (double separators).
- Access/call chaining is restricted and ambiguous.
- Missing terminals (`id`, `chars`), comments, `null`, comparisons, logical ops, and `fn { ... }` sugar.

## What to verify or update
If you need authoritative Lexy guidance, check:
- How Lexy models whitespace/comments and whether it has built‑in “token” vs “rule” separation patterns.
- Whether Lexy provides an expression‑precedence helper and how to model prefix/postfix chains.
- How Lexy recommends resolving ambiguity with lookahead or prioritized choices.

## Files referenced
- `../frost/design/design.frst`
- `../frost/design/grammer.ebnf`
- `../frost/design/gpt-issues.md`

