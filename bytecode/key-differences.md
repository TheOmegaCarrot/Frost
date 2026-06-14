Key differences of the bytecode reimplementation against the reference:

Set:

- `foo.bar` becomes "error on missing" instead of "null on missing"
    - `foo['bar']` stays "null on missing"
- Lambdas now properly discard `_` repeatably
    - `fn _, a, _ -> a` was an error but now must be accepted
- A ninth `Opaque` type is added, replacing the foreign value system, and spiritually in-line with Lua's light userdata
- Floats like `.5` are accepted. This has no ambiguity with dot access as a digit is not valid on the rhs of dot acess
- Trailing commas are accepted in more places where another element _may_ follow
