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

- **Source pane** (left): the original Frost source with line numbers and syntax
  highlighting (via the tree-sitter grammar).
- **AST pane** (right): a collapsible tree matching the structure of `frost -d`.
  Each node shows its label and source range. Structural wrapper nodes
  (Expression, Bindings, LHS, RHS, etc.) appear in italic gray.
- The divider between panes is draggable to resize.

### Bidirectional highlighting

- **Hover source -> AST**: hovering a source character highlights all AST nodes
  whose source range includes that character. The innermost (most specific) node
  gets a strong highlight; enclosing ancestor nodes get a lighter highlight. The
  innermost node is scrolled into view.

- **Hover AST -> source**: hovering an AST node highlights its source range in
  the source pane.

### Click to pin

Click a source character or AST node to pin the highlight (amber). The pin
persists while you move between panes. Clicking an AST node also scrolls the
source pane to the corresponding code. Click the same element again to unpin.
Press Escape to clear the pin.

### Search

- **Source search**: case-insensitive substring search. Matches are highlighted
  in amber and the view scrolls to the first match.
- **AST search**: filters the tree by node label. Non-matching subtrees are
  collapsed and matching nodes are highlighted. The original expand/collapse
  state is restored when the search is cleared.

Press Escape to clear both searches.

### Tree navigation

- Click a toggle arrow to expand or collapse a single node.
- Alt+click a toggle arrow to recursively expand or collapse the entire subtree.
- **Expand All** / **Collapse All** buttons in the toolbar.

### Source range warnings

The tool automatically validates AST source ranges and flags issues:

- **no_range**: a real AST node missing its source range (red).
- **Inverted range**: end position before start position.
- **Range beyond source**: line or column exceeds the actual source text.
- **Child exceeds parent**: a child node's range extends outside its parent's.
- **Overlapping siblings**: two sibling nodes with partially overlapping ranges.

Warnings appear as amber text next to the flagged node. The toolbar shows a
summary count, or a green checkmark if no warnings are found.

## How it works

The C++ side (`astviz.cpp`) parses the input file with `frst::parse_file()`,
walks the AST via `children()`, and serializes the tree to JSON using
Boost.JSON. Syntax highlighting is produced by parsing the source a second time
with the tree-sitter Frost grammar and running the `highlights.scm` queries.
The HTML template (`template.html`) and highlight queries are embedded at
compile time via `#embed`. The output is a single HTML file with the JSON data
and template concatenated together.

The JavaScript builds a reverse index from source positions to AST node IDs at
page load, enabling efficient bidirectional lookups on hover.

## Build

`astviz` is built automatically as part of the normal CMake build:

```bash
cmake --build build -j4
```

The binary is placed at `build/astviz`.
