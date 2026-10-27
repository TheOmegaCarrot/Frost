# frost-arcade

A terminal arcade whose games are Frost modules, edited while they run. It is a
canary: a small, unpublished application written against Frost's public
embedding API, to find where embedding is awkward. It is the game-tick shape of
host: the host owns time and state, and each frame runs short Frost functions.

```bash
cargo run -p frostlang-canary-arcade                 # the menu of games
cargo run -p frostlang-canary-arcade -- blocks       # one game
cargo run -p frostlang-canary-arcade -- --root my-games snake
cargo run -p frostlang-canary-arcade -- --headless 600 snake   # no terminal
```

While a game runs, `esc` returns to the menu, `ctrl-r` restarts it, and
`ctrl-c` quits. Every other key goes to the game.

## Editing a game as it runs

Save any file under the root and the game picks it up within a frame, keeping
its state:

- If the new version does not load, the game runs on the version before it,
  with the error shown, until the file is fixed.
- If `update` or `draw` fails, the game stops where it was, with the error
  shown. Fixing the code resumes it from the last good state; `ctrl-r` starts
  over instead.
- If an edit changes the shape of the state, `ctrl-r` to start fresh.

Loading happens on a thread of its own: see the `ingest` module.

## Writing a game

A game is a directory under the root holding `main.frst`. Modules import one
another by path from the root, so `lib/draw.frst` is `import('lib.draw')`, and
games share modules that way. They can also import the standard library's
contained modules (`std.math`, `std.random`, `std.string`, and so on).

`main.frst` exports four things:

```frost
export def meta = {title: 'Snake', width: 50, height: 19, tick_rate: 30}
export defn init(env) -> ...           # the first state
export defn update(state, input) -> ... # the next state
export defn draw(state) -> [...]        # what to show
```

- `meta.width` and `meta.height` are the game's size in cells. `tick_rate` is
  updates per second, 1 to 120; it defaults to 30. A frame is drawn after each
  round of updates.
- `init` receives `{width, height, seed}`.
- `update` receives the state and `{tick, keys, seed}`: `tick` counts updates
  from 0, and `keys` lists the keys pressed since the last update, oldest
  first, such as `'up'`, `'space'`, `'enter'`, `'tab'`, `'backspace'`, or a
  character like `'p'`.
- Each `seed` is a fresh Int. For randomness, make an engine from it where it
  is needed, `import('std.random').seed(input.seed)`, rather than keeping one
  in the state: an engine changes as it draws, so the state would no longer
  be a plain value.
- `draw` returns an Array of draw ops, painted in order, clipped to the game:
  - `{x, y, text}` writes `text` rightward from `x`, `y`.
  - `{x, y, w, h, fill}` fills the rectangle with the one character `fill`.

  Either may add `fg` and `bg`, each a color name (`black`, `dark_grey`,
  `grey`, `white`, and `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`,
  each also as `dark_red` and so on) or `[r, g, b]`.

`print` goes to the console under the game, which shows the last few lines.

Each call may make a few million calls of its own before it is stopped, so a
runaway recursion stops the game rather than hanging it.
