# Documentation Todo List

Track progress migrating from hand-written markdown (`docs/stdlib/`) to structured `.frst` sources (`docs/src/stdlib/`).

## Stdlib Modules (std)

- [x] cli
- [x] datetime
- [x] encoding
- [x] fs
- [x] io
- [x] json
- [x] math
- [x] os
- [x] random
- [x] regex
- [x] string

## Extension Modules (ext)

- [x] compression
- [x] hash
- [x] http
- [x] msgpack
- [x] sqlite
- [x] toml
- [x] unsafe

## Standalone Pages / Builtins

- [x] collections
- [x] debug
- [x] foreign-values
- [x] functions
- [x] mutable-cell
- [x] operators
- [x] output
- [x] streams
- [x] strings
- [x] types
- [x] README / index (stdlib landing page)

## Generator / Backend

- [x] Generator: tree walker with CLI
- [X] Generator: resolve @ref links in prose strings
- [x] Markdown backend: core primitives
- [x] Markdown backend: tables
- [X] Markdown backend: robust ref-to-link resolution
- [ ] Test harness: walk doc tree, execute testable examples, assert results
- [ ] Ref validation: check that all `see_also` and `@ref` paths resolve

## Schema

- [x] SCHEMA.md: module, entry, type, child, param, content array, page, table

## Post-Migration

- [X] Replace `docs/stdlib/*.md` with generated output
- [X] Replace `generate-index.frst` with generator-produced index
- [X] Update `docs/stdlib/README.md` to be generated from `placeholder_name` in root.frst
- [X] Remove hand-written markdown files
- [ ] CI: regenerate docs and fail if output differs from committed (ensures data and output stay in sync)

## Not Yet Planned

- [x] Language reference (`docs/src/lang/`) -- 7 files: root, lexical, types, expressions (+ operators, match, iterative), statements, modules
- [x] Introduction (`docs/src/intro.frst`)
- [ ] REPL `help()` integration
- [ ] HTML backend
- [ ] Full-text + embedding search backend
