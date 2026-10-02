//! `std.regex`: matching, replacing, splitting, and scanning text by regular
//! expression.
//!
//! Patterns use the syntax of the `regex-lite` crate: Rust's regex syntax,
//! without backreferences or lookaround, and with ASCII-only character classes.
//! A replacement refers to groups as `$1` or `${name}`, the whole match as `$0`,
//! and writes a literal `$` as `$$`.
//!
//! Each function compiles its pattern on every call. `regex.compile(pattern)`
//! compiles one once, returning a Map of the same functions, less the pattern.

use std::sync::{Arc, OnceLock};

use regex_lite::Regex;

use crate::{FrostError, FrostResult, FrostType, NativeCtx, Param, Params, StdlibModule, Value};

/// The `std.regex` module: testing, replacing, splitting, and scanning text by
/// regular expression, and compiling a pattern for reuse.
///
/// It only computes: it reads and changes nothing outside the script.
pub fn regex() -> StdlibModule {
    StdlibModule::new(
        "regex",
        Value::map([
            ("matches", one_text("regex.matches", Pattern::matches)),
            ("contains", one_text("regex.contains", Pattern::contains)),
            ("replace", replacer("regex.replace", Count::All)),
            (
                "replace_first",
                replacer("regex.replace_first", Count::First),
            ),
            ("replace_with", replace_with()),
            ("split", one_text("regex.split", Pattern::split)),
            (
                "scan_matches",
                one_text("regex.scan_matches", Pattern::scan_matches),
            ),
            ("compile", compile()),
        ]),
    )
}

/// The text of a type-checked String argument.
fn string_arg(arg: &Value) -> &str {
    arg.as_str().expect("type-checked as a String")
}

// --- Compiled patterns ---

/// A compiled pattern, and the operations every function performs with one.
struct Pattern {
    regex: Regex,
    /// The pattern anchored at both ends, for `matches`; built on first use.
    whole: OnceLock<Regex>,
}

/// How many matches a replacement replaces.
#[derive(Clone, Copy)]
enum Count {
    All,
    First,
}

impl Pattern {
    /// `pattern`, the pattern argument of `function`, compiled.
    fn new(function: &str, pattern: &str) -> Result<Self, FrostError> {
        let regex = Regex::new(pattern).map_err(|err| {
            FrostError::from_string(format!(
                "Function {function} got an invalid pattern {}: {err}",
                Value::from(pattern).to_debug_string()
            ))
        })?;
        Ok(Self {
            regex,
            whole: OnceLock::new(),
        })
    }

    fn matches(&self, text: &str) -> Value {
        // Anchored, the pattern must match all of the text. Comparing a match's
        // span with the text's would not do: `a|ab` finds only `a` in `ab`.
        // The pattern is already valid alone, which wrapping it relies on: an
        // unbalanced one such as `a)|(?:b` would otherwise become valid, with
        // another meaning.
        let whole = self.whole.get_or_init(|| {
            Regex::new(&format!(r"\A(?:{})\z", self.regex.as_str()))
                .expect("a valid pattern stays valid anchored")
        });
        Value::Bool(whole.is_match(text))
    }

    fn contains(&self, text: &str) -> Value {
        Value::Bool(self.regex.is_match(text))
    }

    fn replace(&self, text: &str, replacement: &str, count: Count) -> Value {
        let replaced = match count {
            Count::All => self.regex.replace_all(text, replacement),
            Count::First => self.regex.replace(text, replacement),
        };
        Value::from(replaced.into_owned())
    }

    fn replace_with(&self, ctx: &mut NativeCtx<'_>, text: &str, callback: &Value) -> FrostResult {
        // Built by hand: the callback can raise, which `Regex::replace_all`'s
        // closure has no way to report.
        let mut replaced = String::with_capacity(text.len());
        let mut copied_to = 0;
        for found in self.regex.find_iter(text) {
            replaced.push_str(&text[copied_to..found.start()]);
            let replacement = ctx.invoke(callback, [Value::from(found.as_str())])?;
            replaced.push_str(&replacement.to_frost_string());
            copied_to = found.end();
        }
        replaced.push_str(&text[copied_to..]);
        Ok(replaced.into())
    }

    fn split(&self, text: &str) -> Value {
        self.regex.split(text).map(Value::from).collect()
    }

    fn scan_matches(&self, text: &str) -> Value {
        // A group's state: whether it took part in the match, and what it matched.
        let group = |found: Option<regex_lite::Match<'_>>| {
            let value = found.map_or(Value::Null, |found| Value::from(found.as_str()));
            (Value::Bool(found.is_some()), value)
        };
        let matches: Vec<Value> = self
            .regex
            .captures_iter(text)
            .map(|captures| {
                let groups: Vec<Value> = (0..captures.len())
                    .map(|index| {
                        let (matched, value) = group(captures.get(index));
                        Value::map([
                            ("matched", matched),
                            ("value", value),
                            (
                                "index",
                                Value::Int(i64::try_from(index).expect("a group index fits")),
                            ),
                        ])
                    })
                    .collect();
                let named: Value = self
                    .regex
                    .capture_names()
                    .flatten()
                    .map(|name| {
                        let (matched, value) = group(captures.name(name));
                        (
                            name.into(),
                            Value::map([("matched", matched), ("value", value)]),
                        )
                    })
                    .collect();
                Value::map([
                    ("full", Value::from(&captures[0])),
                    ("groups", groups.into()),
                    ("named", named),
                ])
            })
            .collect();
        Value::map([
            ("found", Value::Bool(!matches.is_empty())),
            (
                "count",
                Value::Int(i64::try_from(matches.len()).expect("a count fits")),
            ),
            ("matches", matches.into()),
        ])
    }
}

// --- Module functions: each compiles its pattern, argument 2 ---

/// A function of text and a pattern, performing `operation`.
fn one_text(name: &'static str, operation: fn(&Pattern, &str) -> Value) -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::STRING),
        Param::of(FrostType::STRING).named("pattern"),
    ]);
    Value::checked_native(name, PARAMS, move |_, args| {
        let pattern = Pattern::new(name, string_arg(&args[1]))?;
        Ok(operation(&pattern, string_arg(&args[0])))
    })
}

fn replacer(name: &'static str, count: Count) -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::STRING),
        Param::of(FrostType::STRING).named("pattern"),
        Param::of(FrostType::STRING).named("replacement"),
    ]);
    Value::checked_native(name, PARAMS, move |_, args| {
        let pattern = Pattern::new(name, string_arg(&args[1]))?;
        Ok(pattern.replace(string_arg(&args[0]), string_arg(&args[2]), count))
    })
}

fn replace_with() -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::STRING),
        Param::of(FrostType::STRING).named("pattern"),
        Param::of(FrostType::FUNCTION).named("callback"),
    ]);
    Value::checked_native("regex.replace_with", PARAMS, |mut ctx, args| {
        let pattern = Pattern::new("regex.replace_with", string_arg(&args[1]))?;
        pattern.replace_with(&mut ctx, string_arg(&args[0]), &args[2])
    })
}

// --- compile: the same functions over one compiled pattern ---

fn compile() -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::STRING).named("pattern")]);
    Value::checked_native("regex.compile", PARAMS, |_, args| {
        let pattern = Arc::new(Pattern::new("regex.compile", string_arg(&args[0]))?);
        Ok(Value::map([
            (
                "matches",
                bound_one_text("compiled_regex.matches", &pattern, Pattern::matches),
            ),
            (
                "contains",
                bound_one_text("compiled_regex.contains", &pattern, Pattern::contains),
            ),
            (
                "replace",
                bound_replacer("compiled_regex.replace", &pattern, Count::All),
            ),
            (
                "replace_first",
                bound_replacer("compiled_regex.replace_first", &pattern, Count::First),
            ),
            ("replace_with", bound_replace_with(&pattern)),
            (
                "split",
                bound_one_text("compiled_regex.split", &pattern, Pattern::split),
            ),
            (
                "scan_matches",
                bound_one_text(
                    "compiled_regex.scan_matches",
                    &pattern,
                    Pattern::scan_matches,
                ),
            ),
        ]))
    })
}

/// A function of text alone, performing `operation` with `pattern`.
fn bound_one_text(
    name: &'static str,
    pattern: &Arc<Pattern>,
    operation: fn(&Pattern, &str) -> Value,
) -> Value {
    const PARAMS: Params = Params::new(&[Param::of(FrostType::STRING)]);
    let pattern = Arc::clone(pattern);
    Value::checked_native(name, PARAMS, move |_, args| {
        Ok(operation(&pattern, string_arg(&args[0])))
    })
}

fn bound_replacer(name: &'static str, pattern: &Arc<Pattern>, count: Count) -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::STRING),
        Param::of(FrostType::STRING).named("replacement"),
    ]);
    let pattern = Arc::clone(pattern);
    Value::checked_native(name, PARAMS, move |_, args| {
        Ok(pattern.replace(string_arg(&args[0]), string_arg(&args[1]), count))
    })
}

fn bound_replace_with(pattern: &Arc<Pattern>) -> Value {
    const PARAMS: Params = Params::new(&[
        Param::of(FrostType::STRING),
        Param::of(FrostType::FUNCTION).named("callback"),
    ]);
    let pattern = Arc::clone(pattern);
    Value::checked_native(
        "compiled_regex.replace_with",
        PARAMS,
        move |mut ctx, args| pattern.replace_with(&mut ctx, string_arg(&args[0]), &args[1]),
    )
}
