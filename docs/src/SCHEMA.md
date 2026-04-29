# Documentation Schema Reference

This document describes the Frost map structures used in `.frst` doc source files under `docs/src/`. It should be sufficient to author a new module's documentation without needing to refer to existing modules.

## Quick Start

A minimal module file looks like this:

```frost
export def mymodule = {
    name: 'mymodule',
    title: 'My Module',
    module_path: 'std.mymodule',
    import_as: 'mm',
    description: [
        'A brief summary of what this module provides.',
    ],
    entries: [
        {
            name: 'do_thing',
            signatures: ['mymodule.do_thing(value)'],
            description: [
                'Does a thing with `value` and returns the result.',
            ],
            body: [
                { code: "mymodule.do_thing(42)", result: '84' },
            ],
        },
    ],
}
```

Export the module under a name matching `name`. The file goes in `docs/src/stdlib/` and is wired into `stdlib/root.frst` via `import`.

## Content Array

Many fields in the schema (`description`, `content`, type/child `description`) are **content arrays**: polymorphic arrays where each element is one of:

| Shape | Description |
|-------|-------------|
| String | A prose paragraph. |
| `{ code, result }` | Testable example. `result` is the `to_string` representation of the last expression's value. |
| `{ code, output }` | Testable example. `output` is expected stdout. |
| `{ code, error }` | Testable example. `error` is a substring of the expected error message. |
| `{ code, illustrative: true }` | Not tested. Shows a pattern or usage that can't be reduced to a simple assertion. |
| `{ title, content }` | A titled sub-section. `content` is a nested content array. |
| `{ list: [...] }` | A bulleted list. Each item is a string. |

All of these can appear in any content array. A description that needs an inline example can have one:

```frost
description: [
    'Returns the absolute form of `path` without resolving symlinks.',
    { code: "fs.absolute('relative/file.txt')", result: '/home/user/relative/file.txt' },
],
```

### Inline formatting in prose strings

Three forms of inline formatting are permitted in prose strings:

- Backtick code spans: `` `value` ``
- Links (external): `[text](url)`
- Links (cross-reference): `` [`parse_int`](@ref std.encoding.parse_int) ``

External and internal links use the same `[text](target)` syntax. Internal cross-references are distinguished by the `@ref ` prefix in the URL position. The display text is explicit — use backtick-wrapped function names, plain text, or whatever fits the prose.

### Code examples

`code` is a string: single-line or multiline. Multiline strings use Frost's triple-quote syntax with Swift-style indentation trimming. The following example results in a string with no leading whitespace in the content:

```frost
body: [
    'some prose',
    {
        code: '''
            def db = sqlite.open_memory()
            db.exec('CREATE TABLE t (id INTEGER PRIMARY KEY)')
            db.exec('INSERT INTO t VALUES (?)', [1])
            ''',
        result: '1',
    },
    'more prose',
],
```

## Module

A module is the top-level unit for a stdlib or extension documentation file. Each module is its own `.frst` file exporting a single map.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `name` | String | yes | Machine name. Used for file paths, anchors, and as the entry prefix in signatures. |
| `title` | String | no | Human-readable display name (e.g. `'SQLite'`, `'TOML'`). Falls back to `name` if absent. |
| `module_path` | String | no | Dot-separated import path (e.g. `'std.encoding'`, `'ext.sqlite'`). Omit for builtin modules that require no import (e.g. streams). Technically optional, but rarely omitted. |
| `import_as` | String | no | Canonical short name shown in the import line (e.g. `'enc'`, `'sqlite'`). When present, the generator emits an import code block at the top of the page. |
| `description` | Content Array | yes | Module-level summary. Rendered immediately after the import line. |
| `content` | Content Array | no | Extended narrative rendered before the API reference. Use for build flags, usage guides, conceptual explanations, etc. |
| `types` | Map | no | Object types returned by this module. Keys are type names. See [Type](#type). |
| `entries` | Array | yes | Top-level functions and constants. See [Entry](#entry). |
| `children` | Map | no | Sub-namespaces (e.g. `hex`, `b64`). Keys are child names. See [Child](#child). |

### Render order

1. Title heading (from `title` or `name`)
2. Import code block (if `import_as` present)
3. `description`
4. `content`
5. `types` (sorted by name, each with its own heading and methods)
6. `entries` (headings use short name, no module prefix)
7. `children` (sorted by name, each with its own heading and entries)

## Entry

A function or constant within a module, type, or child.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `name` | String | yes | Short name (e.g. `'fmt_int'`, `'exec'`). Used in headings. |
| `kind` | String | no | Set to `'constant'` for non-callable entries (no signatures). Omit for functions. |
| `signatures` | Array | yes* | How to call the function, with the full qualified path as the user would type it (e.g. `'encoding.fmt_int(number, base)'`). Multiple entries for overloaded signatures. *Omit for constants. |
| `description` | Content Array | yes | Brief description of what the function does. Used as the summary for machine consumers (REPL help, search indexing). |
| `body` | Content Array | no | Everything after the description: examples, extended narrative, sub-sections, caveats. Author controls the flow: examples can be interspersed with prose wherever they read best. |
| `params` | Array | no | Parameter documentation. Omit when self-evident from the signature and description. See [Param](#param). |
| `see_also` | Array | no | Cross-reference paths. See [Cross-References](#cross-references). |

All optional fields should be omitted entirely when empty (not set to `[]` or `null`).

### Heading conventions

- **Module entries**: heading shows just `name` (e.g. `open`). No module prefix: the page context makes it clear.
- **Child entries**: heading shows `childname.entryname` (e.g. `hex.encode`).
- **Type entries**: heading shows `typename.entryname` (e.g. `db.exec`).
- **Signatures**: always show the full qualified path as the user would type it (e.g. `encoding.hex.encode(s)`, `db.exec(sql)`).

### Constants

A constant entry has `kind: 'constant'` and no `signatures`:

```frost
{
    name: 'version',
    kind: 'constant',
    description: [
        'A `String` containing the SQLite library version (e.g. `"3.51.3"`).',
    ],
},
```

## Type

An object type returned by module functions (e.g. the database connection from `sqlite.open()`). Rendered at the module level with its own heading, before top-level entries.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `name` | String | yes | Machine name, used as prefix for method headings and signatures (e.g. `'db'`). |
| `title` | String | no | Display name for the section heading (e.g. `'Database Connection'`). Falls back to `name`. |
| `description` | Content Array | no | Introductory content explaining what this type represents, where it comes from, and how it's used. |
| `entries` | Array | yes | Methods on this type. Same shape as [Entry](#entry). |

The type description should tell the reader where the type comes from (which functions return it) so the docs stand alone without needing to read the constructor functions first.

## Child

A sub-namespace within a module (e.g. `hex`, `b64`, `url` in the encoding module). Rendered with its own section heading, after top-level entries.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `name` | String | yes | Machine name, used as prefix for entry headings and signatures (e.g. `'hex'`). |
| `title` | String | no | Display name for the section heading. Falls back to `name`. |
| `description` | Content Array | no | Brief introduction for the sub-namespace. |
| `entries` | Array | yes | Functions in this child. Same shape as [Entry](#entry). |

Children model sub-maps of the module's import result. `encoding.hex.encode(s)` is the `encode` function on the `hex` sub-map of the encoding module.

## Param

Documents a single function parameter.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `name` | String | yes | Parameter name, matching the signature. |
| `desc` | String | no | Short description. |

Omit the entire `params` array when the parameters are self-evident from the signature and description. Include it when parameters have non-obvious constraints (e.g. valid ranges, type requirements), when the function takes many parameters, or if the parameter can be given a useful name that is not redundant.
The params should agree with the signatures.

## Page

A standalone concept page for content that doesn't fit the module schema: no functions, no import, just prose. Examples: foreign values, streams (builtins with no import), conceptual guides.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `name` | String | yes | Machine name. Used for file paths and anchors. |
| `title` | String | no | Display name for the page heading. Falls back to `name`. |
| `content` | Content Array | yes | The page body. |

Pages are wired into the doc tree alongside modules. The generator renders them as a heading followed by the content array.

## Cross-References

`see_also` entries and `@ref` links in prose use absolute paths from the doc root.

Path structure: `namespace.module.target`

Examples:
- `std.encoding.parse_int` -- a top-level entry
- `std.encoding.hex.decode` -- a child entry
- `ext.sqlite.db.exec` -- a type method
- `lang.match.guards` -- a language reference section (future)

Paths are resolved by the generator and rendered by each backend (markdown: anchor links, REPL: help path hints).

## File Organization

```
docs/src/
    docroot.frst              # assembles the full doc tree
    intro.frst                # introduction (stub)
    generator.frst            # tree walker, calls backend
    lang/
        root.frst             # language reference (stub)
    stdlib/
        root.frst             # merges std and ext module imports
        encoding.frst         # one file per module
        sqlite.frst
    backends/
        markdown.frst         # markdown output backend
```

Each module file exports a single named value matching the module name:

```frost
export def encoding = { name: 'encoding', ... }
```

`stdlib/root.frst` merges module exports with `+`:

```frost
export def std = (
    import('encoding')
    + import('collections')
    # ...
)
```

To add a new module: create `stdlib/modulename.frst`, export a map following the [Module](#module) schema, and add `+ import('modulename')` to the appropriate namespace in `root.frst`.
