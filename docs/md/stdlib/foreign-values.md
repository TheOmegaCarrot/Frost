# Foreign Values

Some extensions work with types that Frost does not natively support. For example, TOML has datetimes and SQLite has BLOBs.

When an extension encounters one of these types, it produces a **foreign value**: a `Function` that carries the original typed data internally.

Foreign values have two uses:

- **Call it** to get a Frost-native approximate representation (often a `String`).
- **Pass it through** back to the originating extension to preserve the original type.

```frost
# TOML example: datetimes are foreign values
def toml = import('ext.toml')
def doc = toml.decode('created = 2024-03-16')
doc.created       # => <Function>
doc.created()     # => "2024-03-16"
toml.encode(doc)  # round-trips as a TOML date, not a string
```

This allows data to pass through Frost code without losing type information, even when Frost has no native representation for the type. Extensions that produce foreign values also accept them back, enabling lossless round-tripping.

User-created functions (e.g., `fn -> '2024-03-16'` for the TOML example) cannot be used in place of a foreign value. Only the extension that defines the type can produce valid foreign values for it. Extensions typically also provide helper functions to explicitly create foreign values from scratch.

Individual extensions provide further documentation on how they handle foreign values.

