//! The arcade: a menu of the games found, and the game being played.
//!
//! The host owns time. One thread polls the keyboard, runs each due update
//! at the game's tick rate, draws, and picks up reloads, all between frames.
//! Loading happens on the ingester's thread, so a reload costs a frame no
//! more than a registry lookup and a check of the game's exports.

use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::Color;
use frostlang::{Value, from_value};
use frostlang_canary_arcade::ingest::{Ingester, LoadError, Module, Registry};

use crate::game::{DrawOp, Env, Failure, Game, Input, Meta, Runner, Shape};
use crate::screen::{Cell, Frame, Screen};

/// Updates a frame may catch up on after falling behind, before the game
/// slows down instead.
const MAX_CATCH_UP: u32 = 4;

/// The lines a game printed most recently, newest last.
pub(crate) type Console = Arc<Mutex<VecDeque<String>>>;

/// How many printed lines the console keeps.
pub(crate) const CONSOLE_LINES: usize = 3;

pub(crate) struct App {
    ingester: Ingester,
    runner: Runner,
    console: Console,
    view: View,
    /// A message for the status line, and when it was shown.
    notice: Option<(String, Instant)>,
}

enum View {
    Menu { selected: usize },
    Playing(Box<Session>),
}

/// A game being played.
struct Session {
    /// The game's main module.
    module: String,
    /// The module's generation that `game` came from.
    generation: u64,
    game: Game,
    /// The state after the last update that succeeded.
    state: Value,
    tick: u64,
    keys: Vec<String>,
    next_tick: Instant,
    /// What the last draw that succeeded returned.
    ops: Vec<DrawOp>,
    /// The failure that stopped the game, until a reload or a restart.
    failure: Option<Failure>,
    /// Why the latest version of the game did not load, while the game runs
    /// on the version before it.
    load_error: Option<String>,
    update_cost: (Duration, u64),
    draw_cost: (Duration, u64),
}

/// What to do after handling a key.
enum Flow {
    Continue,
    Quit,
}

impl App {
    pub(crate) fn new(ingester: Ingester, runner: Runner, console: Console) -> Self {
        Self {
            ingester,
            runner,
            console,
            view: View::Menu { selected: 0 },
            notice: None,
        }
    }

    /// Start the game `name` (its directory), rather than showing the menu.
    pub(crate) fn play(&mut self, name: &str) -> Result<(), String> {
        let registry = self.ingester.registry();
        let session = self.start(&registry, &format!("{name}.main"))?;
        self.view = View::Playing(Box::new(session));
        Ok(())
    }

    /// Play the current game for `ticks` updates without a terminal, as fast
    /// as it runs, pressing a scripted key now and then; drawing after every
    /// update. A report of how it went, or the failure that stopped it.
    pub(crate) fn headless(&mut self, ticks: u64) -> Result<String, String> {
        const KEYS: [&str; 8] = ["up", "right", "down", "space", "left", "x", "down", "z"];
        let View::Playing(session) = &mut self.view else {
            return Err("headless play needs a game named".to_owned());
        };
        let mut update = (Duration::ZERO, Duration::ZERO);
        let mut draw = (Duration::ZERO, Duration::ZERO);
        for tick in 0..ticks {
            if tick % 5 == 0 {
                let key = KEYS[usize::try_from(tick / 5).unwrap_or(0) % KEYS.len()];
                session.keys.push(key.to_owned());
            }
            session.next_tick = Instant::now();
            session.advance(&mut self.runner);
            if let Some(failure) = &session.failure {
                return Err(format!("failed at tick {tick}:\n{}", failure.message));
            }
            for (total, (time, _)) in [
                (&mut update, session.update_cost),
                (&mut draw, session.draw_cost),
            ] {
                total.0 += time;
                total.1 = total.1.max(time);
            }
        }
        let per_tick = |(total, max): (Duration, Duration)| {
            format!(
                "{}us mean, {}us max",
                (total / u32::try_from(ticks.max(1)).unwrap_or(u32::MAX)).as_micros(),
                max.as_micros()
            )
        };
        let meta = &session.game.meta;
        let mut picture = Frame::new(meta.width, meta.height);
        let mut clipped = Clipped {
            frame: &mut picture,
            x0: 0,
            y0: 0,
            width: i32::from(meta.width),
            height: i32::from(meta.height),
        };
        for op in &session.ops {
            clipped.op(op);
        }
        Ok(format!(
            "{} ticks of {}\nupdate: {}\ndraw: {} ({} ops last)\n{}\n{}",
            ticks,
            meta.title,
            per_tick(update),
            per_tick(draw),
            session.ops.len(),
            picture.to_text(),
            session.load_error.as_deref().unwrap_or("")
        ))
    }

    pub(crate) fn run(&mut self, screen: &mut Screen) -> io::Result<()> {
        loop {
            let wake = match &self.view {
                View::Playing(session) if session.failure.is_none() => session.next_tick,
                // Nothing runs on a timer: wake only to see reloads.
                _ => Instant::now() + Duration::from_millis(100),
            };
            while event::poll(wake.saturating_duration_since(Instant::now()))? {
                if let Event::Key(key) = event::read()?
                    && matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
                    && let Flow::Quit = self.key(key)
                {
                    return Ok(());
                }
            }
            let registry = self.ingester.registry();
            if let View::Playing(session) = &mut self.view {
                if let Some(notice) = session.reload(&registry) {
                    self.notice = Some((notice, Instant::now()));
                }
                session.advance(&mut self.runner);
            }
            let mut frame = screen.frame()?;
            self.paint(&mut frame, &registry);
            screen.show(frame)?;
        }
    }

    fn key(&mut self, key: KeyEvent) -> Flow {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match (&mut self.view, key.code) {
            (_, KeyCode::Char('c')) if ctrl => return Flow::Quit,
            (View::Playing(_), KeyCode::Esc) => self.view = View::Menu { selected: 0 },
            (View::Playing(session), KeyCode::Char('r')) if ctrl => {
                let notice = match session.restart(&mut self.runner) {
                    Ok(()) => "restarted".to_owned(),
                    Err(failure) => failure.message,
                };
                self.notice = Some((notice, Instant::now()));
            }
            (View::Playing(session), code) => session.keys.extend(key_name(code)),
            (View::Menu { .. }, KeyCode::Esc | KeyCode::Char('q')) => return Flow::Quit,
            (View::Menu { selected }, KeyCode::Up) => *selected = selected.saturating_sub(1),
            (View::Menu { selected }, KeyCode::Down) => *selected += 1,
            (View::Menu { selected }, KeyCode::Enter) => {
                let registry = self.ingester.registry();
                if let Some(module) = games(&registry).nth(*selected) {
                    let name = module.name().to_owned();
                    match self.start(&registry, &name) {
                        Ok(session) => self.view = View::Playing(Box::new(session)),
                        Err(error) => self.notice = Some((error, Instant::now())),
                    }
                }
            }
            (View::Menu { .. }, _) => {}
        }
        Flow::Continue
    }

    /// A new session of the game whose main module is `module`.
    fn start(&mut self, registry: &Registry, module: &str) -> Result<Session, String> {
        let found = registry
            .get(module)
            .ok_or_else(|| format!("there is no module {module}"))?;
        let (exports, load_error) = match found.outcome() {
            Ok(exports) => (exports, None),
            Err(_) => {
                let failure = load_failure(registry, module);
                match found.last_good() {
                    Some(exports) => (exports, Some(failure)),
                    None => return Err(failure.replace('\n', " ")),
                }
            }
        };
        let game = Game::from_exports(exports)?;
        let mut session = Session {
            module: module.to_owned(),
            generation: found.generation(),
            game,
            state: Value::Null,
            tick: 0,
            keys: Vec::new(),
            next_tick: Instant::now(),
            ops: Vec::new(),
            failure: None,
            load_error,
            update_cost: (Duration::ZERO, 0),
            draw_cost: (Duration::ZERO, 0),
        };
        session
            .restart(&mut self.runner)
            .map_err(|failure| format!("`init` failed: {}", failure.message))?;
        Ok(session)
    }

    fn paint(&self, frame: &mut Frame, registry: &Registry) {
        let notice = self
            .notice
            .as_ref()
            .filter(|(_, shown)| shown.elapsed() < Duration::from_secs(3))
            .map(|(notice, _)| notice.as_str());
        let help = match &self.view {
            View::Menu { selected } => {
                paint_menu(frame, registry, *selected);
                "up/down choose  enter play  q quit".to_owned()
            }
            View::Playing(session) => {
                let console = self.console.lock().unwrap_or_else(PoisonError::into_inner);
                session.paint(frame, &console);
                session.status()
            }
        };
        let y = i32::from(frame.height()) - 1;
        let help_at = match notice {
            Some(notice) => {
                let notice = format!(" {notice} ");
                frame.text(0, y, &notice, Color::Black, Color::Yellow);
                i32::try_from(notice.chars().count()).unwrap_or(0) + 1
            }
            None => 0,
        };
        frame.text(help_at, y, &help, Color::DarkGrey, Color::Reset);
    }
}

/// Why the module `name` failed to load, put as briefly as can be: when it
/// failed for importing a module that failed, the error of each module at the
/// root of the failure, rather than the chain of import errors leading to it.
fn load_failure(registry: &Registry, name: &str) -> String {
    let mut roots = Vec::new();
    let mut seen = Vec::new();
    find_root_failures(registry, name, &mut seen, &mut roots);
    roots
        .iter()
        .map(|module| match module.outcome() {
            Err(error) => format!("{}: {}", module.name(), brief(error)),
            Ok(_) => unreachable!("only failed modules are roots"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn find_root_failures<'a>(
    registry: &'a Registry,
    name: &str,
    seen: &mut Vec<String>,
    roots: &mut Vec<&'a Module>,
) {
    if seen.iter().any(|seen| seen == name) {
        return;
    }
    seen.push(name.to_owned());
    let Some(module) = registry.get(name) else {
        return;
    };
    if module.outcome().is_ok() {
        return;
    }
    let failed_imports: Vec<&str> = module
        .imports()
        .filter(|import| {
            registry
                .get(import)
                .is_some_and(|import| import.outcome().is_err())
        })
        .collect();
    if failed_imports.is_empty() {
        roots.push(module);
    }
    for import in failed_imports {
        find_root_failures(registry, import, seen, roots);
    }
}

/// `error` without the source snippets: the message of each diagnostic and
/// where it points.
fn brief(error: &LoadError) -> String {
    match error {
        // TODO: this picks lines out of the narrated rendering, for want of a
        // diagnostic's message and location as data.
        LoadError::Compile(diagnostics) => diagnostics
            .iter()
            .flat_map(|diagnostic| {
                let narrated = diagnostic.render_narrated();
                let message = narrated.lines().next().unwrap_or_default().to_owned();
                let labels: Vec<String> = narrated
                    .lines()
                    .filter_map(|line| line.trim().strip_prefix("label at "))
                    .map(|label| format!("  at {label}"))
                    .collect();
                [message].into_iter().chain(labels)
            })
            .collect::<Vec<_>>()
            .join("\n"),
        other => other.to_string(),
    }
}

impl Session {
    /// Start the game over from `init`.
    fn restart(&mut self, runner: &mut Runner) -> Result<(), Failure> {
        let env = Env {
            width: self.game.meta.width,
            height: self.game.meta.height,
            seed: seed(),
        };
        self.state = self.game.init(runner, &env)?;
        self.tick = 0;
        self.keys.clear();
        self.next_tick = Instant::now();
        self.failure = None;
        Ok(())
    }

    /// Switch to the latest version of the game, if there is a new one,
    /// returning a notice of what happened.
    fn reload(&mut self, registry: &Registry) -> Option<String> {
        let Some(module) = registry.get(&self.module) else {
            if self.load_error.is_none() {
                self.load_error = Some(format!("{} was removed", self.module));
            }
            return None;
        };
        if module.generation() == self.generation {
            return None;
        }
        self.generation = module.generation();
        let loaded = module
            .outcome()
            .map_err(|_| load_failure(registry, &self.module))
            .and_then(Game::from_exports);
        match loaded {
            Ok(game) => {
                self.game = game;
                self.load_error = None;
                // The fix may be in the code that failed: try it again.
                let resumed = self.failure.take().is_some();
                self.next_tick = Instant::now();
                Some(
                    if resumed {
                        "reloaded, resuming"
                    } else {
                        "reloaded"
                    }
                    .to_owned(),
                )
            }
            Err(error) => {
                self.load_error = Some(error);
                Some("reload failed".to_owned())
            }
        }
    }

    /// Run every update now due, then draw.
    fn advance(&mut self, runner: &mut Runner) {
        if self.failure.is_some() {
            return;
        }
        let tick = self.game.tick();
        let now = Instant::now();
        if now.saturating_duration_since(self.next_tick) > tick * MAX_CATCH_UP {
            self.next_tick = now;
        }
        while self.next_tick <= now {
            let input = Input {
                tick: self.tick,
                keys: std::mem::take(&mut self.keys),
                seed: seed(),
            };
            // A copy stays behind so that a failed update loses nothing.
            match self.game.update(runner, self.state.clone(), &input) {
                Ok(state) => self.state = state,
                Err(failure) => {
                    self.failure = Some(failure);
                    return;
                }
            }
            self.update_cost = runner.last;
            self.tick += 1;
            self.next_tick += tick;
        }
        match self.game.draw(runner, self.state.clone()) {
            Ok(ops) => self.ops = ops,
            Err(failure) => self.failure = Some(failure),
        }
        self.draw_cost = runner.last;
    }

    fn paint(&self, frame: &mut Frame, console: &VecDeque<String>) {
        let meta = &self.game.meta;
        let (width, height) = (i32::from(meta.width), i32::from(meta.height));
        // The game, the console, and the status line.
        let needed = (width, height + console_height(console) + 1);
        let available = (i32::from(frame.width()), i32::from(frame.height()));
        if available.0 < needed.0 || available.1 < needed.1 {
            frame.text(
                0,
                0,
                &format!(
                    "The terminal is too small: {} needs {}x{}",
                    meta.title, needed.0, needed.1
                ),
                Color::Reset,
                Color::Reset,
            );
            return;
        }
        let left = (available.0 - needed.0) / 2;
        let top = (available.1 - needed.1) / 2;
        let mut clipped = Clipped {
            frame,
            x0: left,
            y0: top,
            width,
            height,
        };
        for op in &self.ops {
            clipped.op(op);
        }
        if let Some(failure) = &self.failure {
            let heading = if failure.aborted {
                "Stopped by a limit. Fix the code to resume, or ctrl-r to restart."
            } else {
                "Failed. Fix the code to resume, or ctrl-r to restart."
            };
            clipped.message(heading, &failure.message, Color::White, Color::DarkRed);
        } else if let Some(error) = &self.load_error {
            clipped.message(
                "The new version did not load; running the one before.",
                error,
                Color::Black,
                Color::DarkYellow,
            );
        }
        for (line, y) in console.iter().zip(top + height..) {
            frame.text(left, y, line, Color::DarkGrey, Color::Reset);
        }
    }

    /// What the status line says while the game is on.
    fn status(&self) -> String {
        format!(
            "{}  tick {}  update {}  draw {}  |  esc menu  ctrl-r restart  ctrl-c quit",
            self.game.meta.title,
            self.tick,
            cost(self.update_cost),
            cost(self.draw_cost),
        )
    }
}

fn console_height(console: &VecDeque<String>) -> i32 {
    i32::try_from(console.len()).expect("the console keeps a few lines")
}

/// A call's cost, as the status line shows it.
fn cost((time, fuel): (Duration, u64)) -> String {
    format!("{:>5}us/{fuel:<6}", time.as_micros())
}

/// The game area of a frame: draw ops land here, clipped to it.
struct Clipped<'a> {
    frame: &'a mut Frame,
    x0: i32,
    y0: i32,
    width: i32,
    height: i32,
}

impl Clipped<'_> {
    fn set(&mut self, x: i32, y: i32, cell: Cell) {
        if (0..self.width).contains(&x) && (0..self.height).contains(&y) {
            self.frame.set(self.x0 + x, self.y0 + y, cell);
        }
    }

    fn op(&mut self, op: &DrawOp) {
        let DrawOp { x, y, fg, bg, .. } = *op;
        match op.shape {
            Shape::Text(ref text) => {
                for (ch, x) in text.chars().zip(x..) {
                    self.set(x, y, Cell { ch, fg, bg });
                }
            }
            Shape::Fill { w, h, fill } => {
                for y in y..y.saturating_add(h) {
                    for x in x..x.saturating_add(w) {
                        self.set(x, y, Cell { ch: fill, fg, bg });
                    }
                }
            }
        }
    }

    /// A heading and a message across the bottom of the game area, wrapped to
    /// its width.
    fn message(&mut self, heading: &str, message: &str, fg: Color, bg: Color) {
        let width = usize::try_from(self.width).unwrap_or(0).max(1);
        let lines: Vec<String> = [heading]
            .into_iter()
            .chain(message.lines())
            .flat_map(|line| wrap(line, width))
            .collect();
        let shown = lines.len().min(usize::try_from(self.height).unwrap_or(0));
        let top = self.height - i32::try_from(shown).expect("a few lines");
        for (line, y) in lines.iter().take(shown).zip(top..) {
            for x in 0..self.width {
                self.set(x, y, Cell { ch: ' ', fg, bg });
            }
            for (ch, x) in line.chars().zip(0..) {
                self.set(x, y, Cell { ch, fg, bg });
            }
        }
    }
}

/// `line` broken into pieces at most `width` characters long.
fn wrap(line: &str, width: usize) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return vec![String::new()];
    }
    chars
        .chunks(width)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

/// Every game's main module: one `main.frst` per game directory.
fn games(registry: &Registry) -> impl Iterator<Item = &Module> {
    registry.modules().filter(|module| {
        module
            .name()
            .strip_suffix(".main")
            .is_some_and(|game| !game.contains('.'))
    })
}

fn paint_menu(frame: &mut Frame, registry: &Registry, selected: usize) {
    let left = 4;
    frame.text(left, 1, "FROST ARCADE", Color::Cyan, Color::Reset);
    frame.text(
        left,
        2,
        "up/down choose  enter play  q quit",
        Color::DarkGrey,
        Color::Reset,
    );
    let count = games(registry).count();
    if count == 0 {
        frame.text(left, 4, "No games found.", Color::Reset, Color::Reset);
    }
    let selected = selected.min(count.saturating_sub(1));
    for ((index, module), y) in games(registry).enumerate().zip(4..) {
        let exports = module.outcome().ok().or(module.last_good());
        let title = exports
            .and_then(|exports| match exports {
                Value::Map(map) => map.get_str("meta").cloned(),
                _ => None,
            })
            .and_then(|meta| from_value::<Meta>(meta).ok())
            .map_or_else(|| module.name().to_owned(), |meta| meta.title);
        let marker = if index == selected { "> " } else { "  " };
        let (fg, note) = match module.outcome() {
            Ok(_) => (Color::Reset, String::new()),
            Err(error) => (
                Color::Yellow,
                format!(
                    "  ({})",
                    error.to_string().lines().next().unwrap_or_default()
                ),
            ),
        };
        frame.text(left, y, &format!("{marker}{title}{note}"), fg, Color::Reset);
    }
}

/// The name a game sees for a key: `up`, `space`, `a`, and so on.
fn key_name(code: KeyCode) -> Option<String> {
    Some(match code {
        KeyCode::Char(' ') => "space".to_owned(),
        KeyCode::Char(ch) => ch.to_string(),
        KeyCode::Up => "up".to_owned(),
        KeyCode::Down => "down".to_owned(),
        KeyCode::Left => "left".to_owned(),
        KeyCode::Right => "right".to_owned(),
        KeyCode::Enter => "enter".to_owned(),
        KeyCode::Tab => "tab".to_owned(),
        KeyCode::Backspace => "backspace".to_owned(),
        _ => return None,
    })
}

/// A seed for a game's randomness, different every time.
fn seed() -> i64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    // Truncating: any 64 bits of it will do.
    nanos as i64
}
