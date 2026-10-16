# tree-sitter-frost

Tree-sitter grammar for the Frost language (`.frst`).

## Develop

From this directory:

```sh
tree-sitter generate
tree-sitter test
tree-sitter parse -c path/to/file.frst
tree-sitter highlight -p . --check path/to/file.frst
```

## Compare Against Frost Parser

The parser in `crates/frostlang-parse` is the source of truth. Print the syntax
tree it gives a script, from the repository root:

```sh
cargo run -p frostlang-cli -- ast path/to/file.frst
```

or, with the CLI already built, `frost ast path/to/file.frst`.

## Neovim Notes

This repository already sets `*.frst` filetype to `frost` in:

- `editor/nvim/ftdetect/frost.lua`

It also provides local Tree-sitter integration files:

- `editor/nvim/after/plugin/frost-treesitter.lua`
- `editor/nvim/queries/frost/highlights.scm`
- `editor/nvim/queries/frost/indents.scm`
- `editor/nvim/queries/frost/folds.scm`
- `editor/nvim/queries/frost/rainbow-delimiters.scm`

The parser config points Neovim at `editor/tree-sitter-frost/src/parser.c`.
