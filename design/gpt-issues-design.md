# Design issues (current)

## Open issues / ambiguities
- Local declaration vs assignment is unclear. Example uses `say_hi = fn () -> { ... }` without `def`; decide whether plain `=` declares in local scope or only assigns, and what happens on assignment to an undefined name.
- Newline handling needs a formal, unambiguous rule (especially inside `()`, `[]`, `{}`) given "newline is whitespace or separator".
- Operator precedence and associativity are not specified.
- `map ... into map` availability for v1 is not fixed (design doc suggests it may be omitted early).
- Map callback return requirement is underspecified: must it be a map literal, or is any map expression allowed?
- Builtin surface area is not defined (`tostring`, `print` formatting behavior, and any others).
- `fn { ... }` appears in the design doc but is marked as a stretch goal; decide whether to keep it in the main spec or move to a future-features section.

## Todo items (ordered by importance)
- Specify declaration vs assignment semantics and syntax (including undefined-name assignment).
- Formalize statement separation and newline rules for parsing.
- Specify operator precedence and associativity.
- Decide v1 support for `map ... into map` and define the accepted return form for map-mapping callbacks.
- Define builtin functions and `print` formatting rules.
- Reconcile `fn { ... }` usage in the design doc with its stretch-goal status.
