# astviz

Interactive AST visualizer for Frost source files. Parses a `.frst` file and
outputs a self-contained HTML file with a two-pane view: source code on the
left, AST tree on the right, with bidirectional hover highlighting.

This is developer tooling for inspecting and verifying AST source ranges. It is
not part of the Frost distribution.

## Usage

```bash
./build/astviz script.frst > output.html
```

Open the resulting HTML file in any modern browser.

## Features

### Two-pane layout

- **Source pane** (left): the original Frost source with line numbers.
- **AST pane** (right): a collapsible tree matching the structure of `frost -d`.
  Each node shows its label and source range. Structural wrapper nodes
  (Expression, Bindings, LHS, RHS, etc.) appear in italic gray.

### Bidirectional highlighting

- **Hover source -> AST**: hovering a source character highlights all AST nodes
  whose source range includes that character. The innermost (most specific) node
  gets a strong highlight; enclosing ancestor nodes get a lighter highlight. The
  innermost node is scrolled into view.

- **Hover AST -> source**: hovering an AST node highlights its source range in
  the source pane.

### Click to pin

Click a source character or AST node to pin the highlight (amber). The pin
persists while you move between panes. Click the same element again to unpin.

## How it works

The C++ side (`astviz.cpp`) parses the input file with `frst::parse_file()`,
walks the AST via `children()`, and serializes the tree to JSON using
Boost.JSON. The HTML template (`template.html`) is embedded at compile time via
`#embed`. The output is a single HTML file with the JSON data and template
concatenated together.

The JavaScript builds a reverse index from source positions to AST node IDs at
page load, enabling efficient bidirectional lookups on hover.

## Build

`astviz` is built automatically as part of the normal CMake build:

```bash
cmake --build build -j4
```

The binary is placed at `build/astviz`.
