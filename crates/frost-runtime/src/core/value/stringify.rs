use std::fmt::Write;

use crate::core::util::identifier::is_identifier_like_and_not_keyword;
use crate::core::{FrostMap, MapKey, Value};

impl Value {
    /// Converts to a compact string representation.
    /// Strings at the top level are unquoted.
    pub fn to_frost_string(&self) -> String {
        let mut buf = String::new();
        stringify(self, &mut buf, &StringifyContext::compact());
        buf
    }

    /// Converts to a pretty-printed string with indentation.
    /// Strings at the top level are unquoted.
    pub fn to_pretty_string(&self) -> String {
        let mut buf = String::new();
        stringify(self, &mut buf, &StringifyContext::pretty());
        buf
    }

    /// Converts to a string representation where all values are quoted/escaped.
    pub fn to_debug_string(&self) -> String {
        let mut buf = String::new();
        stringify(self, &mut buf, &StringifyContext::debug());
        buf
    }
}

struct StringifyContext {
    in_structure: bool,
    pretty: bool,
    depth: usize,
}

impl StringifyContext {
    fn compact() -> Self {
        Self {
            in_structure: false,
            pretty: false,
            depth: 0,
        }
    }

    fn pretty() -> Self {
        Self {
            in_structure: false,
            pretty: true,
            depth: 0,
        }
    }

    fn debug() -> Self {
        Self {
            in_structure: true,
            pretty: false,
            depth: 0,
        }
    }

    fn nested(&self) -> Self {
        Self {
            in_structure: true,
            pretty: self.pretty,
            depth: self.depth + 1,
        }
    }

    fn indent(&self) -> String {
        " ".repeat(self.depth * 4)
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
        Value::NativeFunction(_) => buf.push_str("<Function>"),
        Value::Closure(_) => buf.push_str("<Function>"),
        Value::Opaque(o) => write!(buf, "<{}>", o.type_name()).unwrap(),
    }
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
    for &byte in bytes.iter() {
        write!(buf, "{byte:02x}").unwrap();
    }
    buf.push('\'');
}

/// Escapes a String for display inside a structure.
/// Printable text passes through as itself, so a non-ASCII character renders as the character it is.
fn escape_string(s: &str, buf: &mut String) {
    buf.push('"');
    for ch in s.chars() {
        match ch {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\n' => buf.push_str("\\n"),
            '\t' => buf.push_str("\\t"),
            '\r' => buf.push_str("\\r"),
            // A control character has no readable spelling, so it escapes as its
            // scalar value: `\u{NN}`, minimal hex digits, matching Rust's `escape_debug`.
            c if c.is_control() => {
                write!(buf, "\\u{{{:x}}}", c as u32).unwrap();
            }
            c => buf.push(c),
        }
    }
    buf.push('"');
}

fn stringify_array(elems: &[Value], buf: &mut String, ctx: &StringifyContext) {
    if elems.is_empty() {
        buf.push_str("[]");
        return;
    }

    let nested = ctx.nested();

    if ctx.pretty {
        buf.push_str("[\n");
        for (i, elem) in elems.iter().enumerate() {
            if i > 0 {
                buf.push_str(",\n");
            }
            buf.push_str(&nested.indent());
            stringify(elem, buf, &nested);
        }
        buf.push('\n');
        buf.push_str(&ctx.indent());
        buf.push(']');
    } else {
        buf.push_str("[ ");
        for (i, elem) in elems.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            stringify(elem, buf, &nested);
        }
        buf.push_str(" ]");
    }
}

fn stringify_map(map: &FrostMap, buf: &mut String, ctx: &StringifyContext) {
    if map.is_empty() {
        buf.push_str("{}");
        return;
    }

    let nested = ctx.nested();

    if ctx.pretty {
        buf.push_str("{\n");
        for (i, (key, value)) in map.iter().enumerate() {
            if i > 0 {
                buf.push_str(",\n");
            }
            buf.push_str(&nested.indent());
            stringify_map_entry(key, value, buf, &nested);
        }
        buf.push('\n');
        buf.push_str(&ctx.indent());
        buf.push('}');
    } else {
        buf.push_str("{ ");
        for (i, (key, value)) in map.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            stringify_map_entry(key, value, buf, &nested);
        }
        buf.push_str(" }");
    }
}

fn stringify_map_entry(key: &MapKey, value: &Value, buf: &mut String, ctx: &StringifyContext) {
    let shorthand_name = match key {
        MapKey::String(s) if ctx.pretty && is_identifier_like_and_not_keyword(s) => {
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
