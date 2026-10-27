//! `frost-arcade`: play the games under a directory, editing them as they run.
//!
//! ```text
//! frost-arcade [--root <dir>] [--headless <ticks>] [<game>]
//! ```
//!
//! The root defaults to this crate's `games` directory. With a game named,
//! it starts that game rather than showing the menu. `--headless` plays the
//! game for that many updates with scripted keys and no terminal, and reports
//! how long its functions took.

mod app;
mod game;
mod screen;

use std::collections::VecDeque;
use std::io;
use std::num::NonZeroU64;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError};

use crossterm::cursor::{Hide, Show};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use frostlang::stdlib::StdlibConfig;
use frostlang::{ImporterBuilder, Stdlib, Vm, VmRuntimeConfiguration};
use frostlang_canary_arcade::ingest::Ingester;

use app::{App, CONSOLE_LINES, Console};
use game::Runner;
use screen::Screen;

/// The calls one run may make: far more than a frame of any sensible game
/// needs, and few enough that a runaway one stops within moments.
const FUEL: NonZeroU64 = NonZeroU64::new(5_000_000).unwrap();

/// How deeply one run's calls may nest.
const MAX_CALL_DEPTH: NonZeroU64 = NonZeroU64::new(10_000).unwrap();

const MAX_IMPORT_DEPTH: NonZeroU64 = NonZeroU64::new(32).unwrap();

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("frost-arcade: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("games");
    let mut game = None;
    let mut headless = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => root = args.next().ok_or("--root needs a directory")?.into(),
            "--headless" => {
                let ticks = args.next().ok_or("--headless needs a number of ticks")?;
                headless = Some(ticks.parse::<u64>().map_err(|error| error.to_string())?);
            }
            _ if game.is_none() && !arg.starts_with('-') => game = Some(arg),
            _ => return Err(format!("unexpected argument '{arg}'")),
        }
    }

    let console: Console = Arc::new(Mutex::new(VecDeque::new()));
    let configuration = VmRuntimeConfiguration::default()
        .with_fuel(Some(FUEL))
        .with_max_call_depth(Some(MAX_CALL_DEPTH))
        .with_max_import_depth(Some(MAX_IMPORT_DEPTH))
        .with_print_sink(Arc::new({
            let console = Arc::clone(&console);
            move |text: &str| {
                let mut console = console.lock().unwrap_or_else(PoisonError::into_inner);
                console.extend(text.lines().map(str::to_owned));
                let excess = console.len().saturating_sub(CONSOLE_LINES);
                console.drain(..excess);
            }
        }));
    let ingester = Ingester::builder(&root)
        .with_importer(
            ImporterBuilder::new().with_stdlib(Stdlib::contained(StdlibConfig::default())),
        )
        .with_configuration(configuration.clone())
        .start()
        .map_err(|error| error.to_string())?;
    let runner = Runner::new(
        Vm::factory()
            .configuration(configuration)
            .with_importer(ingester.importer()),
    );

    let mut app = App::new(ingester, runner, console);
    if let Some(game) = game {
        app.play(&game)?;
    }
    if let Some(ticks) = headless {
        let report = app.headless(ticks)?;
        println!("{report}");
        return Ok(());
    }
    let _terminal = Terminal::enter().map_err(|error| error.to_string())?;
    app.run(&mut Screen::new())
        .map_err(|error| error.to_string())
}

/// The terminal in raw mode on the alternate screen, until dropped.
struct Terminal;

impl Terminal {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, Hide)?;
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            default_hook(info);
        }));
        Ok(Self)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        restore();
    }
}

fn restore() {
    let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
    let _ = disable_raw_mode();
}
