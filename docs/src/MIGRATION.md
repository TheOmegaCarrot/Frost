# Documentation Migration Checklist

Track progress migrating from hand-written markdown (`docs/stdlib/`) to structured `.frst` sources (`docs/src/stdlib/`).

## Stdlib Modules (std)

- [x] encoding
- [x] io
- [ ] cli
- [X] collections
- [ ] datetime
- [ ] debug
- [ ] fs
- [ ] functions (core HOFs: `tap`, `identity`, `try_call`, etc.)
- [ ] json
- [ ] math
- [ ] mutable-cell
- [ ] operators (predefined operator functions: `plus`, `minus`, etc.)
- [ ] os
- [ ] output (print, mprint, etc.)
- [ ] random
- [ ] regex
- [ ] string
- [ ] strings
- [ ] types (type-checking functions: `is_string`, `to_int`, etc.)
- [ ] unsafe

## Extension Modules (ext)

- [x] sqlite
- [X] compression
- [ ] hash
- [ ] http
- [ ] msgpack
- [ ] toml

## Standalone Pages

- [x] streams
- [x] foreign-values
- [ ] README / index (stdlib landing page)

## Generator / Backend

- [x] Generator: tree walker with CLI
- [ ] Generator: parse @ref links into a single format for backends
- [x] Markdown backend: core primitives
- [ ] Markdown backend: robust ref-to-link resolution
- [ ] Markdown backend: module heading capitalization via `title` field (done for migrated modules)
- [ ] Test harness: walk doc tree, execute testable examples, assert results
- [ ] Ref validation: check that all `see_also` and `@ref` paths resolve

## Schema

- [x] SCHEMA.md: module, entry, type, child, param, content array, page

## Post-Migration

- [ ] Replace `docs/stdlib/*.md` with generated output
- [ ] Replace `generate-index.frst` with generator-produced index
- [ ] Update `docs/stdlib/README.md` to be generated from `placeholder_name` in root.frst
- [ ] Remove hand-written markdown files
- [ ] CI: regenerate docs and fail if output differs from committed (ensures data and output stay in sync)

## Not Yet Planned

- Language reference (`docs/src/lang/`)
- Introduction (`docs/src/intro.frst`)
- REPL `help()` integration
- HTML backend
- Full-text search backend
