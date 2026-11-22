//! How `frost` ends when a script calls `fatal(value)`.
//!
//! - A String is printed as the message, and the exit status is 1.
//! - A Map whose only keys are `message` and `code`: `message`, a String, is
//!   printed if present, else nothing is; `code`, an Int from 1 to 255, is the
//!   exit status, 1 if absent.
//! - Anything else is printed in its debug form, and the exit status is 1,
//!   with a line saying why when `code` is the problem. So a mistake, such as a
//!   misspelled key, shows rather than ending silently.
//!
//! A message is printed to standard error as given, followed by a newline
//! unless it ends with one, with no label or trace.

use std::num::NonZeroU8;
use std::process::ExitCode;

use frostlang::{FrostMap, MapKey, Value};

const MESSAGE: &str = "message";
const CODE: &str = "code";

/// End the process as `fatal(value)` asks.
pub(crate) fn exit(value: Value) -> ExitCode {
    let Ending { message, code } = ending(value);
    if let Some(message) = message {
        if message.ends_with('\n') {
            eprint!("{message}");
        } else {
            eprintln!("{message}");
        }
    }
    ExitCode::from(code.get())
}

/// What to print, if anything, and the exit status.
struct Ending {
    message: Option<String>,
    code: NonZeroU8,
}

/// The ending `fatal(value)` asks for.
fn ending(value: Value) -> Ending {
    let unconventional = match &value {
        Value::String(text) => {
            return Ending {
                message: Some(text.as_str().to_string()),
                code: NonZeroU8::MIN,
            };
        }
        Value::Map(map) => match map_ending(map) {
            Ok(ending) => return ending,
            Err(unconventional) => unconventional,
        },
        _ => Unconventional::Shape,
    };
    let mut message = value.to_debug_string();
    if matches!(unconventional, Unconventional::Code) {
        message.push_str("\nfatal: `code` must be an Int from 1 to 255, so the exit status is 1");
    }
    Ending {
        message: Some(message),
        code: NonZeroU8::MIN,
    }
}

/// Why a value given to `fatal` does not follow the convention.
enum Unconventional {
    /// It is not a String, nor a Map of only `message` and a valid `code`.
    Shape,
    /// It is such a Map, but for its `code`.
    Code,
}

/// The ending for a Map given to `fatal`, if it follows the convention.
fn map_ending(map: &FrostMap) -> Result<Ending, Unconventional> {
    let only_known_keys = map.iter().all(
        |(key, _)| matches!(key, MapKey::String(name) if [MESSAGE, CODE].contains(&name.as_str())),
    );
    if !only_known_keys {
        return Err(Unconventional::Shape);
    }
    let message = match map.get(&MapKey::from(MESSAGE)) {
        None => None,
        Some(Value::String(text)) => Some(text.as_str().to_string()),
        Some(_) => return Err(Unconventional::Shape),
    };
    let code = match map.get(&MapKey::from(CODE)) {
        None => NonZeroU8::MIN,
        Some(Value::Int(code)) => u8::try_from(*code)
            .ok()
            .and_then(NonZeroU8::new)
            .ok_or(Unconventional::Code)?,
        Some(_) => return Err(Unconventional::Code),
    };
    Ok(Ending { message, code })
}
