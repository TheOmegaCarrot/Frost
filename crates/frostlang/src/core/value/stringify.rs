use std::fmt::Write;

use crate::core::util::identifier::is_identifier_like_and_not_keyword;
use crate::core::{FrostMap, FrostOpaque, MapKey, RenderLimits, Value};

impl Value {
    /// Converts to a compact string representation, within the default [`RenderLimits`].
    /// Strings at the top level are unquoted.
    pub fn to_frost_string(&self) -> String {
        self.to_frost_string_within(RenderLimits::new())
    }

    /// Converts to a compact string representation, within `limits`.
    /// Strings at the top level are unquoted.
    pub fn to_frost_string_within(&self, limits: RenderLimits) -> String {
        render(self, limits, Style::Compact)
    }

    /// Converts to a pretty-printed string with indentation, within the default
    /// [`RenderLimits`]. Strings at the top level are unquoted.
    pub fn to_pretty_string(&self) -> String {
        self.to_pretty_string_within(RenderLimits::new())
    }

    /// Converts to a pretty-printed string with indentation, within `limits`.
    /// Strings at the top level are unquoted.
    pub fn to_pretty_string_within(&self, limits: RenderLimits) -> String {
        render(self, limits, Style::Pretty)
    }

    /// Converts to a compact string representation with Strings quoted and escaped, even
    /// at the top level, within the default [`RenderLimits`].
    pub fn to_debug_string(&self) -> String {
        self.to_debug_string_within(RenderLimits::new())
    }

    /// Converts to a compact string representation with Strings quoted and escaped, even
    /// at the top level, within `limits`.
    pub fn to_debug_string_within(&self, limits: RenderLimits) -> String {
        render(self, limits, Style::Debug)
    }
}

#[derive(PartialEq)]
enum Style {
    Compact,
    Pretty,
    Debug,
}

fn render(value: &Value, limits: RenderLimits, style: Style) -> String {
    let rendering = Rendering {
        pretty: style == Style::Pretty,
        max_depth: limits.max_depth.unwrap_or(usize::MAX),
        max_len: limits.max_len.unwrap_or(usize::MAX),
    };
    let ctx = StringifyContext {
        rendering: &rendering,
        in_structure: style == Style::Debug,
        depth: 0,
    };
    let mut buf = String::new();
    stringify(value, &mut buf, &ctx);
    buf
}

/// What holds for the whole of one value's rendering: its layout and its limits, each
/// `usize::MAX` where there is none.
struct Rendering {
    pretty: bool,
    max_depth: usize,
    max_len: usize,
}

/// Where in the value being rendered a part of it is.
struct StringifyContext<'a> {
    rendering: &'a Rendering,
    in_structure: bool,
    depth: usize,
}

impl StringifyContext<'_> {
    fn nested(&self) -> Self {
        Self {
            rendering: self.rendering,
            in_structure: true,
            depth: self.depth + 1,
        }
    }

    fn pretty(&self) -> bool {
        self.rendering.pretty
    }

    fn push_indent(&self, buf: &mut String) {
        buf.extend(std::iter::repeat_n(' ', self.depth * 4));
    }

    /// Whether a container at this depth renders as `[...]` or `{...}` in place of its
    /// contents.
    fn past_max_depth(&self) -> bool {
        self.depth > self.rendering.max_depth
    }

    /// Whether the rest of a container renders as `...`, `buf` being the text so far.
    fn at_max_len(&self, buf: &str) -> bool {
        buf.len() >= self.rendering.max_len
    }
}

fn stringify(value: &Value, buf: &mut String, ctx: &StringifyContext) {
    match value {
        Value::Null => buf.push_str("null"),
        Value::Bool(b) => buf.push_str(if *b { "true" } else { "false" }),
        // unwrap because writing to a string is infallible
        Value::Int(i) => write!(buf, "{i}").unwrap(),
        Value::Float(f) => stringify_float(f.get(), buf),
        Value::String(s) => stringify_string(s, buf, ctx),
        Value::Bytes(b) => stringify_bytes(b, buf),
        Value::Array(arr) => stringify_array(arr.as_slice(), buf, ctx),
        Value::Map(map) => stringify_map(map, buf, ctx),
        Value::NativeFunction(_) | Value::Closure(_) => buf.push_str("<Function>"),
        Value::Opaque(o) => stringify_opaque(&**o, buf),
    }
}

/// `<TypeName>`, or `<TypeName: approximation>` with the approximation escaped
/// as a String's contents are, so it cannot break a pretty layout.
fn stringify_opaque(opaque: &dyn FrostOpaque, buf: &mut String) {
    buf.push('<');
    buf.push_str(opaque.type_name());
    if let Some(approximation) = opaque.try_to_string() {
        buf.push_str(": ");
        escape_chars(&approximation, buf);
    }
    buf.push('>');
}

fn stringify_float(f: f64, buf: &mut String) {
    let mut ryu_buf = ryu::Buffer::new();
    buf.push_str(ryu_buf.format(f));
}

fn stringify_string(s: &str, buf: &mut String, ctx: &StringifyContext) {
    if ctx.in_structure {
        escape_string(s, buf);
    } else {
        buf.push_str(s);
    }
}

/// Bytes always render in Bytes-literal form, `x'6869'`, whatever the context, so
/// binary is never mistaken for text: hex pairs, lowercase, inside `x'...'`.
fn stringify_bytes(bytes: &[u8], buf: &mut String) {
    buf.push_str("x'");
    for &byte in bytes {
        write!(buf, "{byte:02x}").unwrap();
    }
    buf.push('\'');
}

// These two unwrap because writing to a String cannot fail.

fn escape_string(s: &str, buf: &mut String) {
    write_quoted_string(s, buf).unwrap();
}

fn escape_chars(s: &str, buf: &mut String) {
    write_escaped_string(s, buf).unwrap();
}

/// Writes `text` the way Frost displays a String inside a structure:
/// in double quotes, escaped as by [`write_escaped_string`].
///
/// ```
/// let mut out = String::new();
/// frostlang::write_quoted_string("say \"hi\"\n", &mut out).unwrap();
/// assert_eq!(out, r#""say \"hi\"\n""#);
/// ```
pub fn write_quoted_string(text: &str, out: &mut impl Write) -> std::fmt::Result {
    out.write_char('"')?;
    write_escaped_string(text, out)?;
    out.write_char('"')
}

/// Writes `text` escaped but unquoted, as Frost displays an Opaque's detail in
/// `<Type: detail>`.
///
/// `"`, `\`, newline, tab, and carriage return escape as `\"`, `\\`, `\n`, `\t`,
/// and `\r`. Any other control character escapes as its scalar value in hex,
/// `\u{1b}`. Everything else, including non-ASCII text, is written as itself.
///
/// ```
/// let mut out = String::new();
/// frostlang::write_escaped_string("tab\there, \u{1b}, é", &mut out).unwrap();
/// assert_eq!(out, r"tab\there, \u{1b}, é");
/// ```
pub fn write_escaped_string(text: &str, out: &mut impl Write) -> std::fmt::Result {
    for ch in text.chars() {
        match ch {
            '"' => out.write_str("\\\"")?,
            '\\' => out.write_str("\\\\")?,
            '\n' => out.write_str("\\n")?,
            '\t' => out.write_str("\\t")?,
            '\r' => out.write_str("\\r")?,
            // Minimal hex digits, matching Rust's `escape_debug`.
            c if c.is_control() => write!(out, "\\u{{{:x}}}", c as u32)?,
            c => out.write_char(c)?,
        }
    }
    Ok(())
}

fn stringify_array(elems: &[Value], buf: &mut String, ctx: &StringifyContext) {
    if elems.is_empty() {
        buf.push_str("[]");
    } else if ctx.past_max_depth() {
        buf.push_str("[...]");
    } else {
        stringify_elements(('[', ']'), elems.iter(), buf, ctx, stringify);
    }
}

fn stringify_map(map: &FrostMap, buf: &mut String, ctx: &StringifyContext) {
    if map.is_empty() {
        buf.push_str("{}");
    } else if ctx.past_max_depth() {
        buf.push_str("{...}");
    } else {
        stringify_elements(('{', '}'), map.iter(), buf, ctx, stringify_map_entry);
    }
}

/// Writes a container at `ctx`'s depth: `elements` between the `open` and `close`
/// brackets, each written by `write`. Once the text reaches the length limit, the rest
/// are written as one `...`.
fn stringify_elements<T>(
    (open, close): (char, char),
    elements: impl Iterator<Item = T>,
    buf: &mut String,
    ctx: &StringifyContext,
    write: impl Fn(T, &mut String, &StringifyContext),
) {
    let nested = ctx.nested();
    let (after_open, separator, before_close) = if ctx.pretty() {
        ("\n", ",\n", "\n")
    } else {
        (" ", ", ", " ")
    };

    buf.push(open);
    buf.push_str(after_open);
    for (i, element) in elements.enumerate() {
        if i > 0 {
            buf.push_str(separator);
        }
        if ctx.pretty() {
            nested.push_indent(buf);
        }
        if nested.at_max_len(buf) {
            buf.push_str("...");
            break;
        }
        write(element, buf, &nested);
    }
    buf.push_str(before_close);
    if ctx.pretty() {
        ctx.push_indent(buf);
    }
    buf.push(close);
}

fn stringify_map_entry((key, value): (&MapKey, &Value), buf: &mut String, ctx: &StringifyContext) {
    let shorthand_name = match key {
        MapKey::String(s) if ctx.pretty() && is_identifier_like_and_not_keyword(s) => {
            Some(s.as_ref())
        }
        _ => None,
    };

    if let Some(name) = shorthand_name {
        buf.push_str(name);
    } else {
        buf.push('[');
        stringify_map_key(key, buf);
        buf.push(']');
    }

    buf.push_str(": ");
    stringify(value, buf, ctx);
}

fn stringify_map_key(key: &MapKey, buf: &mut String) {
    match key {
        MapKey::Bool(b) => buf.push_str(if *b { "true" } else { "false" }),
        MapKey::Int(i) => write!(buf, "{i}").unwrap(),
        MapKey::Float(f) => stringify_float(f.get(), buf),
        MapKey::String(s) => escape_string(s, buf),
        MapKey::Bytes(b) => stringify_bytes(b, buf),
    }
}
