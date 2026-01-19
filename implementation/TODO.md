# Concrete

## Internal

- `Value::clone`
    - explicit deep copy
    - For preventing escape of mutable references from `mutable_cell`

## Builtin

- `mutable_cell`
    - return map with get and set keys
    - values are magic closures that manage a shared mutable cell
    - get explicitly copies out of the cell (clone)
- More complete string operations
    - `split`
    - `join`
    - `containment`
    - `prefix`/`suffix`
- Math operations
    - modulus (New % operator?)
- JSON encode/decode


# Conceptual

- More range operations
    - Zip
- Filesystem operations
    - magic builtin functions return map to magic closures managing a file
- Module system
    - `import foo`
        - search for `foo.frst` relative to importing script
        - run that file with a fresh environment, and copy exported names to table `foo`
    - `export def`
        - like a normal def, but it gets exported when the script is imported
        - only valid at top-level
