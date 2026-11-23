//! How a process ends for a script that called `fatal(value)`: the convention
//! [`Exit`](crate::Exit)'s `Termination` follows, documented there.

use std::io::{self, Write};
use std::num::NonZeroU8;

use frostlang::{FrostMap, MapKey, Value};

const MESSAGE: &str = "message";
const CODE: &str = "code";

/// The exit status for `fatal(value)`.
pub(crate) fn code(value: &Value) -> NonZeroU8 {
    match interpret(value) {
        Interpretation::Conventional { code, .. } => code,
        Interpretation::OtherShape | Interpretation::InvalidCode => NonZeroU8::MIN,
    }
}

/// Print what `fatal(value)` asks for to standard error, if anything. A
/// failure to write is ignored: there is nowhere left to report it, and the
/// exit status still stands.
pub(crate) fn print(value: &Value) {
    match interpret(value) {
        Interpretation::Conventional { message, .. } => {
            if let Some(message) = message {
                print_message(message);
            }
        }
        Interpretation::OtherShape => print_message(&value.to_debug_string()),
        Interpretation::InvalidCode => print_message(&format!(
            "{}\nfatal: `code` must be an Int from 1 to 255, so the exit status is 1",
            value.to_debug_string()
        )),
    }
}

/// Print `message` as given, followed by a newline unless it ends with one.
fn print_message(message: &str) {
    let newline = if message.ends_with('\n') { "" } else { "\n" };
    let _ = write!(io::stderr(), "{message}{newline}");
}

/// What a value given to `fatal` asks for, found without rendering it.
enum Interpretation<'a> {
    /// A String, or a Map of only a String `message` and a valid `code`.
    Conventional {
        message: Option<&'a str>,
        code: NonZeroU8,
    },
    /// Neither a String nor a Map of only a String `message` and a `code`.
    OtherShape,
    /// A Map of only a String `message` and a `code`, but for its `code`.
    InvalidCode,
}

fn interpret(value: &Value) -> Interpretation<'_> {
    match value {
        Value::String(text) => Interpretation::Conventional {
            message: Some(text.as_str()),
            code: NonZeroU8::MIN,
        },
        Value::Map(map) => interpret_map(map),
        _ => Interpretation::OtherShape,
    }
}

fn interpret_map(map: &FrostMap) -> Interpretation<'_> {
    let only_known_keys = map.iter().all(
        |(key, _)| matches!(key, MapKey::String(name) if [MESSAGE, CODE].contains(&name.as_str())),
    );
    if !only_known_keys {
        return Interpretation::OtherShape;
    }
    let message = match map.get(&MapKey::from(MESSAGE)) {
        None => None,
        Some(Value::String(text)) => Some(text.as_str()),
        Some(_) => return Interpretation::OtherShape,
    };
    let code = match map.get(&MapKey::from(CODE)) {
        None => Some(NonZeroU8::MIN),
        Some(Value::Int(code)) => u8::try_from(*code).ok().and_then(NonZeroU8::new),
        Some(_) => None,
    };
    match code {
        Some(code) => Interpretation::Conventional { message, code },
        None => Interpretation::InvalidCode,
    }
}
