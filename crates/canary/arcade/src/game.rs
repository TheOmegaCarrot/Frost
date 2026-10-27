//! What a game is to the host: the module `<game>.main`, exporting `meta`,
//! `init`, `update`, and `draw`. See the README for the contract as a game
//! author sees it.

use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::style::Color;
use frostlang::{
    Closure, FrostError, IdleVm, RunErrorKind, Value, VmFactory, from_value, to_value,
};
use serde::{Deserialize, Serialize};

/// A game's exports, checked.
pub(crate) struct Game {
    pub(crate) meta: Meta,
    init: Arc<Closure>,
    update: Arc<Closure>,
    draw: Arc<Closure>,
}

/// What a game says about itself.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Meta {
    pub(crate) title: String,
    pub(crate) width: u16,
    pub(crate) height: u16,
    /// Updates per second.
    #[serde(default = "Meta::default_tick_rate")]
    pub(crate) tick_rate: u32,
}

impl Meta {
    fn default_tick_rate() -> u32 {
        30
    }
}

/// The exports a game must have, before their functions are checked.
#[derive(Deserialize)]
struct Exports {
    meta: Meta,
    init: Value,
    update: Value,
    draw: Value,
}

impl Game {
    /// The game a module's exports make, or why they make none.
    pub(crate) fn from_exports(exports: &Value) -> Result<Self, String> {
        let exports: Exports = from_value(exports.clone()).map_err(|error| error.to_string())?;
        let closure = |name: &str, value: Value| match value {
            Value::Closure(closure) => Ok(closure),
            other => Err(format!(
                "Export `{name}` must be a Function written in Frost, got {}",
                other.type_name()
            )),
        };
        if exports.meta.tick_rate == 0 || exports.meta.tick_rate > 120 {
            return Err(format!(
                "`meta.tick_rate` must be from 1 to 120, got {}",
                exports.meta.tick_rate
            ));
        }
        Ok(Self {
            meta: exports.meta,
            init: closure("init", exports.init)?,
            update: closure("update", exports.update)?,
            draw: closure("draw", exports.draw)?,
        })
    }

    /// The time between updates.
    pub(crate) fn tick(&self) -> Duration {
        Duration::from_secs(1) / self.meta.tick_rate
    }

    pub(crate) fn init(&self, runner: &mut Runner, env: &Env) -> Result<Value, Failure> {
        let env = to_value(env).expect("the environment is plain data");
        runner.call(&self.init, [env])
    }

    pub(crate) fn update(
        &self,
        runner: &mut Runner,
        state: Value,
        input: &Input,
    ) -> Result<Value, Failure> {
        let input = to_value(input).expect("the input is plain data");
        runner.call(&self.update, [state, input])
    }

    pub(crate) fn draw(&self, runner: &mut Runner, state: Value) -> Result<Vec<DrawOp>, Failure> {
        let ops = runner.call(&self.draw, [state])?;
        from_value(ops).map_err(|error| Failure {
            message: format!("`draw` returned something the host cannot draw: {error}"),
            aborted: false,
        })
    }
}

/// What `init` receives.
#[derive(Serialize)]
pub(crate) struct Env {
    pub(crate) width: u16,
    pub(crate) height: u16,
    /// Fresh for every `init`, to seed a game's randomness.
    pub(crate) seed: i64,
}

/// What `update` receives.
#[derive(Serialize)]
pub(crate) struct Input {
    /// Counts updates since `init`, from 0.
    pub(crate) tick: u64,
    /// The keys pressed since the last update, oldest first.
    pub(crate) keys: Vec<String>,
    /// Fresh for every update, to seed the update's randomness, so that a
    /// game's state need hold no random engine.
    pub(crate) seed: i64,
}

/// One thing to draw, in game coordinates: `x` rightward and `y` downward from
/// the top left, in cells.
//
// Read through `RawOp` rather than as an untagged enum, whose errors say only
// that nothing matched, naming the Rust type.
#[derive(Debug, Deserialize)]
#[serde(try_from = "RawOp")]
pub(crate) struct DrawOp {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) shape: Shape,
    pub(crate) fg: Color,
    pub(crate) bg: Color,
}

#[derive(Debug)]
pub(crate) enum Shape {
    /// Characters rightward.
    Text(String),
    /// A rectangle, every cell one character.
    Fill { w: i32, h: i32, fill: char },
}

/// A draw op as a game writes it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOp {
    x: i32,
    y: i32,
    text: Option<String>,
    fill: Option<char>,
    w: Option<i32>,
    h: Option<i32>,
    #[serde(default)]
    fg: Paint,
    #[serde(default)]
    bg: Paint,
}

impl TryFrom<RawOp> for DrawOp {
    type Error = String;

    fn try_from(op: RawOp) -> Result<Self, String> {
        let shape = match op {
            RawOp {
                text: Some(text),
                fill: None,
                w: None,
                h: None,
                ..
            } => Shape::Text(text),
            RawOp {
                text: None,
                fill: Some(fill),
                w: Some(w),
                h: Some(h),
                ..
            } => Shape::Fill { w, h, fill },
            RawOp {
                text: None,
                fill: Some(_),
                ..
            } => return Err("a `fill` op needs `w` and `h`".to_owned()),
            RawOp { text: Some(_), .. } => {
                return Err("a `text` op has no `fill`, `w`, or `h`".to_owned());
            }
            RawOp { .. } => return Err("a draw op needs `text` or `fill`".to_owned()),
        };
        Ok(Self {
            x: op.x,
            y: op.y,
            shape,
            fg: op.fg.0,
            bg: op.bg.0,
        })
    }
}

/// A color as a game writes it: a name, or `[r, g, b]`.
#[derive(Deserialize)]
#[serde(try_from = "Value")]
struct Paint(Color);

/// The terminal's own color.
impl Default for Paint {
    fn default() -> Self {
        Self(Color::Reset)
    }
}

const COLOR_NAMES: [(&str, Color); 16] = [
    ("black", Color::Black),
    ("dark_grey", Color::DarkGrey),
    ("grey", Color::Grey),
    ("white", Color::White),
    ("red", Color::Red),
    ("dark_red", Color::DarkRed),
    ("green", Color::Green),
    ("dark_green", Color::DarkGreen),
    ("yellow", Color::Yellow),
    ("dark_yellow", Color::DarkYellow),
    ("blue", Color::Blue),
    ("dark_blue", Color::DarkBlue),
    ("magenta", Color::Magenta),
    ("dark_magenta", Color::DarkMagenta),
    ("cyan", Color::Cyan),
    ("dark_cyan", Color::DarkCyan),
];

impl TryFrom<Value> for Paint {
    type Error = String;

    fn try_from(value: Value) -> Result<Self, String> {
        let channel = |value: &Value| match value {
            Value::Int(n) => u8::try_from(*n).ok(),
            _ => None,
        };
        match &value {
            Value::String(name) => COLOR_NAMES
                .iter()
                .find(|(known, _)| *known == &**name)
                .map(|&(_, color)| Self(color))
                .ok_or_else(|| {
                    let names: Vec<&str> = COLOR_NAMES.iter().map(|(name, _)| *name).collect();
                    format!(
                        "unknown color '{name}': expected one of {}",
                        names.join(", ")
                    )
                }),
            Value::Array(rgb) => match rgb.as_slice() {
                [r, g, b] => match (channel(r), channel(g), channel(b)) {
                    (Some(r), Some(g), Some(b)) => Ok(Self(Color::Rgb { r, g, b })),
                    _ => Err("each of a color's [r, g, b] is an Int from 0 to 255".to_owned()),
                },
                _ => Err("a color Array is [r, g, b]".to_owned()),
            },
            other => Err(format!(
                "a color is a name or [r, g, b], got {}",
                other.type_name()
            )),
        }
    }
}

/// A call into a game that failed.
#[derive(Debug, Clone)]
pub(crate) struct Failure {
    pub(crate) message: String,
    /// Stopped by a runtime limit, rather than failing by itself.
    pub(crate) aborted: bool,
}

/// Runs a game's functions, recycling one Vm, and times them.
pub(crate) struct Runner {
    factory: VmFactory,
    idle: Option<IdleVm>,
    /// How long the last call took, and the fuel it used.
    pub(crate) last: (Duration, u64),
}

impl Runner {
    pub(crate) fn new(factory: VmFactory) -> Self {
        Self {
            factory,
            idle: None,
            last: (Duration::ZERO, 0),
        }
    }

    fn call(
        &mut self,
        function: &Arc<Closure>,
        args: impl IntoIterator<Item = Value>,
    ) -> Result<Value, Failure> {
        let function = Arc::clone(function);
        let vm = match self.idle.take() {
            Some(idle) => idle.build(function),
            None => self.factory.build(function),
        };
        let start = Instant::now();
        let outcome = vm.run_with_args(args);
        let elapsed = start.elapsed();
        match outcome {
            Ok(result) => {
                self.last = (elapsed, result.fuel_consumed());
                let parts = result.into_parts();
                self.idle = Some(parts.idle_vm);
                Ok(parts.tail)
            }
            Err(error) => {
                self.last = (elapsed, error.fuel_consumed());
                let aborted = matches!(error.kind(), RunErrorKind::Aborted(_));
                let (error, idle) = error.into_parts();
                self.idle = Some(idle);
                Err(Failure::from_error(&error, aborted))
            }
        }
    }
}

impl Failure {
    fn from_error(error: &FrostError, aborted: bool) -> Self {
        Self {
            message: error.with_backtrace().to_string(),
            aborted,
        }
    }
}
