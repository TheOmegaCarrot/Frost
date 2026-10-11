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

use std::sync::OnceLock;

use regex_lite::Regex;

use crate::native::{Bundle, Function};
use crate::{
    FrostError, FrostResult, FrostString, NativeCtx, StdlibModule, Value, frost_bundle,
    frost_closure, frost_fn,
};

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
            ("replace_with", replace_with_frost()),
            ("split", one_text("regex.split", Pattern::split)),
            (
                "scan_matches",
                one_text("regex.scan_matches", Pattern::scan_matches),
            ),
            ("compile", compile_frost()),
        ]),
    )
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
        // another meaning. So wrapping fails only when the pattern ends in a
        // verbose-mode `#` comment, which swallows the closing `)\z`; ending the
        // line first ends the comment, and verbose mode ignores the line break.
        let whole = self.whole.get_or_init(|| {
            let pattern = self.regex.as_str();
            Regex::new(&format!(r"\A(?:{pattern})\z"))
                .or_else(|_| Regex::new(&format!("\\A(?:{pattern}\n)\\z")))
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

    fn replace_with(
        &self,
        ctx: &mut NativeCtx<'_>,
        text: &str,
        callback: &Function,
    ) -> FrostResult {
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
    frost_closure!(name, move |#[frost(unnamed)] text: FrostString,
                               pattern: FrostString|
          -> FrostResult {
        let pattern = Pattern::new(name, &pattern)?;
        Ok(operation(&pattern, &text))
    })
}

fn replacer(name: &'static str, count: Count) -> Value {
    frost_closure!(name, move |#[frost(unnamed)] text: FrostString,
                               pattern: FrostString,
                               replacement: FrostString|
          -> FrostResult {
        let pattern = Pattern::new(name, &pattern)?;
        Ok(pattern.replace(&text, &replacement, count))
    })
}

#[frost_fn(name = "regex.replace_with")]
fn replace_with(
    #[frost(ctx)] ctx: &mut NativeCtx<'_>,
    #[frost(unnamed)] text: FrostString,
    pattern: FrostString,
    callback: Function,
) -> FrostResult {
    let pattern = Pattern::new("regex.replace_with", &pattern)?;
    pattern.replace_with(ctx, &text, &callback)
}

// --- compile: the same functions over one compiled pattern ---

#[frost_fn(name = "regex.compile")]
fn compile(pattern: FrostString) -> FrostResult {
    let pattern = Pattern::new("regex.compile", &pattern)?;
    Ok(Bundle::new(Compiled(pattern))
        .with(Compiled::frost_functions)
        .build())
}

/// A compiled pattern, whose functions are the module's less the pattern.
struct Compiled(Pattern);

#[frost_bundle(prefix = "compiled_regex")]
impl Compiled {
    fn matches(&self, #[frost(unnamed)] text: FrostString) -> Value {
        self.0.matches(&text)
    }

    fn contains(&self, #[frost(unnamed)] text: FrostString) -> Value {
        self.0.contains(&text)
    }

    fn replace(&self, #[frost(unnamed)] text: FrostString, replacement: FrostString) -> Value {
        self.0.replace(&text, &replacement, Count::All)
    }

    fn replace_first(
        &self,
        #[frost(unnamed)] text: FrostString,
        replacement: FrostString,
    ) -> Value {
        self.0.replace(&text, &replacement, Count::First)
    }

    fn replace_with(
        &self,
        #[frost(ctx)] ctx: &mut NativeCtx<'_>,
        #[frost(unnamed)] text: FrostString,
        callback: Function,
    ) -> FrostResult {
        self.0.replace_with(ctx, &text, &callback)
    }

    fn split(&self, #[frost(unnamed)] text: FrostString) -> Value {
        self.0.split(&text)
    }

    fn scan_matches(&self, #[frost(unnamed)] text: FrostString) -> Value {
        self.0.scan_matches(&text)
    }
}
