# tree-sitter-frost

Tree-sitter grammar for the Frost language (`.frst`).

## Develop

From this directory:

```sh
tree-sitter generate
tree-sitter test
tree-sitter parse -c ../../frost-scripts/hello.frst
tree-sitter highlight --check ../../frost-scripts/hello.frst
```

## Compare Against Frost Parser

Use the interpreter as a source of truth:

```sh
./build/frost -d -e 'def x = {foo: 1, [2+3]: 9}; x.foo'
./build/frost -d path/to/file.frst
```

## Neovim Notes

This repository already sets `*.frst` filetype to `frost` in:

- `editor/nvim/ftdetect/frost.lua`

It also provides local Tree-sitter integration files:

- `editor/nvim/after/plugin/frost-treesitter.lua`
- `editor/nvim/queries/frost/highlights.scm`

The parser config points Neovim at `editor/tree-sitter-frost/src/parser.c`.
