//! Errors for source the lexer cannot read.

use std::ops::Range;

use crate::ast::SourceSpan;
use crate::lex::{FormatStringEnd, scan_format_string, skip_interpolation};
use crate::parse::Diagnostic;
use crate::parse::hints::{
    BITWISE_HELP, CONDITIONAL_HELP, FORMAT_STRING_HELP, GUARD_HELP, INEQUALITY_HELP, LAMBDA_HELP,
    LINE_CONTINUATION_HELP, POWER_HELP, float_point_help, raw_string_help,
};
use crate::parse::match_expr::TYPE_CONSTRAINTS;
use crate::parse::strings::ESCAPES;

const FALLBACK_HELP: &str = "for a fallback when a value is null, use `or`: `a or b`";

/// The error for source the lexer cannot read at `span`, a range of `src`, which
/// starts at `base_offset` in the whole source.
pub(crate) fn unreadable(src: &str, span: Range<usize>, base_offset: usize) -> Diagnostic {
    let shifted =
        |range: Range<usize>| SourceSpan::from(range.start + base_offset..range.end + base_offset);
    let rest = &src[span.start..];

    if let Some((open, quote)) = unclosed_interpolation(rest) {
        let open = span.start + open;
        return Diagnostic::at(
            "unclosed interpolation in format String",
            shifted(open..open + 2),
            "this `${` is not closed",
        )
        .with_help(Some(format!(
            "inside `${{...}}`, each `{{` needs a `}}`, and a `{quote}` starts a nested String"
        )));
    }

    let line_start = src[..span.start]
        .rfind('\n')
        .map_or(0, |newline| newline + 1);
    let line_end = rest
        .find('\n')
        .map_or(src.len(), |newline| span.start + newline);
    let line = src[line_start..line_end].trim_end_matches('\r');
    if let Some(diagnostic) =
        invalid_bytes(&line[span.start - line_start..], span.start + base_offset)
    {
        return diagnostic;
    }
    let starts_file = span.start + base_offset == 0;
    let line_breaks = line_end < src.len();
    let reading = unreadable_on_line(
        line,
        span.start - line_start..span.end - line_start,
        starts_file,
        line_breaks,
    );
    let (span, label) = match reading.unclosed {
        Some(opener) => (
            span.start..span.start + opener.len(),
            format!("this `{opener}` is not closed"),
        ),
        None => (span, "unrecognized".to_owned()),
    };
    Diagnostic::at(reading.message, shifted(span), label).with_help(reading.help)
}

/// What an error says of source the lexer could not read.
struct Reading<'a> {
    message: String,
    /// The opener of a literal the source leaves unclosed, as written
    unclosed: Option<&'a str>,
    help: Option<String>,
}

/// For `rest`, the source from a format String's `$` onward, the index of an
/// interpolation's `${` that never closes, when the String's quote stands later on
/// the `${`'s line; with the quote of the String nested in the interpolation that
/// swallowed its closing `}`.
///
/// In `$'${ {a: 1 }'`, the last quote looks like the String's closer, but the lexer
/// read it as opening a String nested in the interpolation.
fn unclosed_interpolation(rest: &str) -> Option<(usize, char)> {
    let quote = rest
        .strip_prefix('$')?
        .bytes()
        .next()
        .filter(|quote| matches!(quote, b'\'' | b'"'))?;
    let FormatStringEnd::UnclosedInterpolation(open) =
        scan_format_string(&rest.as_bytes()[2..], quote)
    else {
        return None;
    };
    let open = 2 + open;
    let after_open = rest[open + 2..].lines().next().unwrap_or_default();
    if !after_open.contains(char::from(quote)) {
        return None;
    }
    // The String's own quote stands later on the line, so the scan opened a String
    // there at the latest.
    let nested_quote = skip_interpolation(rest.as_bytes(), open + 2)
        .err()
        .flatten()
        .unwrap_or(quote);
    Some((open, char::from(nested_quote)))
}

/// The error for `literal`, the source from a Bytes literal's `x` to the end of its
/// line, which starts at `start` in the whole source, when the lexer could not read it.
/// The error names the literal's first fault and labels it.
fn invalid_bytes(literal: &str, start: usize) -> Option<Diagnostic> {
    const PAIRS_HELP: &str = "a Bytes literal holds pairs of hex digits, like `x'00ff'`";
    let quote = literal
        .strip_prefix('x')?
        .chars()
        .next()
        .filter(|quote| matches!(quote, '\'' | '"'))?;
    const NO_SPACES_HELP: &str =
        "a Bytes literal holds pairs of hex digits with no spaces, like `x'00ff'`";
    let body_start = 2;
    let at = |range: Range<usize>| SourceSpan::from(start + range.start..start + range.end);

    // A missing closing quote is the fault before anything in the body, which may
    // well be the code after the literal.
    let Some(close) = literal[body_start..].find(quote).map(|i| body_start + i) else {
        return Some(
            Diagnostic::at(
                "unclosed Bytes literal",
                at(0..body_start),
                format!("this `x{quote}` is not closed"),
            )
            .with_help(Some(format!(
                "a Bytes literal ends with `{quote}` on the same line"
            ))),
        );
    };
    let fault = literal[body_start..close]
        .char_indices()
        .map(|(i, c)| (body_start + i, c))
        .find(|&(_, c)| !c.is_ascii_hexdigit());
    let (message, span, label, help) = match fault {
        // The lexer reads any even count of digits, so the count is odd:
        // the last digit has no pair.
        None if (close - body_start) % 2 == 1 => (
            "odd number of hex digits in Bytes literal".to_owned(),
            at(close - 1..close),
            "this digit has no pair".to_owned(),
            PAIRS_HELP.to_owned(),
        ),
        None => return None,
        Some((i, ' ')) => (
            "space in Bytes literal".to_owned(),
            at(i..i + 1),
            "not a hex digit".to_owned(),
            NO_SPACES_HELP.to_owned(),
        ),
        Some((i, c)) if c.is_ascii_whitespace() => (
            format!("invalid {} in Bytes literal", describe_char(c)),
            at(i..i + 1),
            "not a hex digit".to_owned(),
            NO_SPACES_HELP.to_owned(),
        ),
        Some((i, c)) => (
            format!("invalid {} in Bytes literal", describe_char(c)),
            at(i..i + c.len_utf8()),
            "not a hex digit".to_owned(),
            PAIRS_HELP.to_owned(),
        ),
    };
    Some(Diagnostic::at(message, span, label).with_help(Some(help)))
}

/// What an error says of source the lexer could not read at `at`, a range of `line`,
/// the line holding it; `starts_file` when that source starts the whole file, and
/// `line_breaks` when a line break ends `line`.
fn unreadable_on_line(
    line: &str,
    at: Range<usize>,
    starts_file: bool,
    line_breaks: bool,
) -> Reading<'_> {
    let (before, rest) = line.split_at(at.start);
    let text = &rest[..at.len().min(rest.len())];
    let mut chars = rest.chars();
    let first = chars.next().unwrap_or(' ');
    let second = chars.next();
    // The name the unreadable character touches, as in `name?` or `Int?`.
    let touched_word = &before[before
        .trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_')
        .len()..];
    let touches_word = touched_word.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_');
    let touches_closer = before.ends_with([')', ']']);
    // Whether an operand comes before the character, spaced or not, as in `a & b`.
    let follows_operand = before.trim_end().ends_with(|c: char| {
        c.is_ascii_alphanumeric() || matches!(c, '_' | ')' | ']' | '}' | '\'' | '"')
    });

    // `1.e3`, whose point has no digits after it: the one number the lexer rejects
    if let Some((whole, exponent)) = text.split_once('.')
        && !whole.is_empty()
        && whole.bytes().all(|byte| byte.is_ascii_digit())
        && exponent.starts_with(['e', 'E'])
    {
        return Reading {
            message: format!("invalid number `{text}`"),
            unclosed: None,
            help: Some(float_point_help(&format!("{whole}.0{exponent}"))),
        };
    }

    let unclosed = |what: &str, opener: usize, help: String| Reading {
        message: format!("unclosed {what}"),
        unclosed: Some(&rest[..opener]),
        help: Some(help),
    };
    let same_line =
        |what: &str, closer: &str| format!("a {what} ends with `{closer}` on the same line");
    match (first, second) {
        ('\'' | '"', _) if rest.starts_with("'''") || rest.starts_with(r#"""""#) => {
            let help = format!("a multiline String ends with `{}`", &rest[..3]);
            return unclosed("multiline String", 3, help);
        }
        // `$'''`, read as the empty format String `$''` and then a quote
        ('\'' | '"', _) if before.ends_with(&format!("${first}{first}")) => {
            let help = format!(
                "Frost has no multiline format String; a format String ends with `{first}` \
                 on the same line"
            );
            return unclosed("String", 1, help);
        }
        // `R'abc`, read as the name `R` and then a String
        ('\'' | '"', _) if touched_word == "R" => {
            return unclosed("String", 1, raw_string_help(first));
        }
        ('\'', _) => return unclosed("String", 1, same_line("String", "'")),
        ('"', _) => return unclosed("String", 1, same_line("String", "\"")),
        ('$', Some('\'')) => return unclosed("format String", 2, same_line("format String", "'")),
        ('$', Some('"')) => return unclosed("format String", 2, same_line("format String", "\"")),
        ('R', Some('\'')) => return unclosed("raw String", 2, same_line("raw String", ")'")),
        ('R', Some('"')) => return unclosed("raw String", 2, same_line("raw String", ")\"")),
        _ => {}
    }

    let help = match (first, second) {
        // `n is Int && n > 1 => 2`, a guard written as a condition
        ('&', Some('&')) if writes_a_guard(before, rest) => Some(GUARD_HELP.to_owned()),
        ('&', Some('&')) => Some("Frost's \"and\" is `and`".to_owned()),
        // `a & b`; a `&` before its operand alone, as in `&x`, is no bitwise operator.
        // In a pattern's type test, as in `n is Int & Float`, no operator would serve.
        ('&', _) if follows_operand && !follows_a_type_test(before) => {
            Some(format!("{BITWISE_HELP}; its \"and\" is `and`"))
        }
        ('^', _) if follows_operand => Some(format!("{BITWISE_HELP}; {POWER_HELP}")),
        // Lua's `~=`
        ('~', Some('=')) => Some(INEQUALITY_HELP.to_owned()),
        // `~a` or `a ~ b`, but not the pattern match `=~` or `!~`
        ('~', _) if !before.ends_with(['=', '!']) => Some(BITWISE_HELP.to_owned()),
        ('?', Some('?' | ':')) => Some(FALLBACK_HELP.to_owned()),
        ('?', Some('.' | '[')) => {
            Some("Frost has no optional chaining; use `x and x.k`".to_owned())
        }
        // `a ? b : c`, or `a?b:c`
        ('?', _)
            if follows_operand
                && (colon_at_this_level(rest) || !(touches_word || touches_closer)) =>
        {
            Some(CONDITIONAL_HELP.to_owned())
        }
        ('?', _)
            if TYPE_CONSTRAINTS
                .iter()
                .any(|(type_name, _)| *type_name == touched_word) =>
        {
            Some("Frost has no optional types; match `null` in an arm of its own".to_owned())
        }
        ('?' | '!', _) if touches_word => Some("Frost names cannot contain `?` or `!`".to_owned()),
        // `f()?` or `f()!`, which unwrap a result in other languages
        ('?' | '!', _) if touches_closer => None,
        ('!', _) => Some("Frost's \"not\" is `not`".to_owned()),
        ('`', _) => Some(FORMAT_STRING_HELP.to_owned()),
        // `\x -> x` or `\(a, b) -> a`, Haskell's lambda
        ('\\', _) if writes_a_lambda(&rest[1..]) => Some(LAMBDA_HELP.to_owned()),
        // `\` ending its line, perhaps before trailing spaces
        ('\\', _) if line_breaks && rest[1..].trim_matches([' ', '\t']).is_empty() => {
            Some(LINE_CONTINUATION_HELP.to_owned())
        }
        // `x\n`, often after a quote that ended a String early
        ('\\', Some(c)) if c == 'u' || ESCAPES.iter().any(|&(escape, _)| escape == c) => {
            Some("a backslash escape works only inside a String".to_owned())
        }
        ('\u{201c}' | '\u{201d}' | '\u{2018}' | '\u{2019}', _) => {
            Some("Frost Strings use straight quotes, `'` or `\"`".to_owned())
        }
        // `λx. x` or `λ x -> x`, a lambda as the lambda calculus writes one
        ('\u{03bb}', _) if writes_a_lambda(&rest['\u{03bb}'.len_utf8()..]) => {
            Some(LAMBDA_HELP.to_owned())
        }
        _ => unicode_help(first, starts_file),
    };
    Reading {
        message: format!("unexpected {}", describe_char(first)),
        unclosed: None,
        help,
    }
}

/// Help for `c`, a character outside ASCII where Frost reads only ASCII, as pasted text
/// often holds; `starts_file` when it starts the whole file.
fn unicode_help(c: char, starts_file: bool) -> Option<String> {
    let help = match c {
        '\u{feff}' if starts_file => {
            "the file starts with a byte order mark; save it as UTF-8 without one"
        }
        // Zero-width characters
        '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{2060}' | '\u{feff}' => {
            "this character is invisible; delete it"
        }
        // A no-break or other wide space
        c if c.is_whitespace() => "this is not a plain space; replace it with one",
        c => {
            if let Some(ascii) = ascii_spelling(c) {
                return Some(format!("Frost's operators are ASCII; write `{ascii}`"));
            }
            if !c.is_alphabetic() {
                return None;
            }
            "Frost names use only ASCII letters, digits, and `_`"
        }
    };
    Some(help.to_owned())
}

/// Whether `before`, the text before a character on its line, ends in a type test, as
/// `n is Int` does.
fn follows_a_type_test(before: &str) -> bool {
    let without_type = before
        .trim_end()
        .trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_');
    without_type
        .trim_end()
        .strip_suffix("is")
        .is_some_and(|rest| rest.is_empty() || rest.ends_with([' ', '\t']))
}

/// Whether `rest`, from a `&&` to the end of its line, and `before`, the text before
/// it, seem to place the `&&` in a `match` arm's pattern: a `=>` ends the arm after
/// it; neither a `=>` nor an `if` stands between it and the arm's start, a `{`, a
/// `,`, or the line's start; and a `{` or `,` there follows a `match` on the line.
/// The lexer's error comes with no tokens, so this reads only the line's text.
fn writes_a_guard(before: &str, rest: &str) -> bool {
    let has_word = |text: &str, word: &str| {
        text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .any(|written| written == word)
    };
    let (arm_start, in_a_match) = match before.rfind(['{', ',']) {
        Some(start) => (start + 1, has_word(&before[..start], "match")),
        // An arm on a line of its own
        None => (0, true),
    };
    let arm_so_far = &before[arm_start..];
    let arm_rest = rest.split([',', '}']).next().unwrap_or_default();
    in_a_match
        && arm_rest.contains("=>")
        && !arm_so_far.contains("=>")
        && !has_word(arm_so_far, "if")
}

/// Whether `text`, following a `λ` or `\`, holds a lambda's parameters, perhaps in
/// parentheses, and then its `.` or `->`, as in `x. x`, ` x, y -> x`, or `(a, b) -> a`.
fn writes_a_lambda(text: &str) -> bool {
    let after_parameters = text.trim_start_matches(|c: char| {
        c.is_ascii_alphanumeric() || matches!(c, '_' | ',' | ' ' | '\t' | '(' | ')')
    });
    after_parameters.len() < text.len()
        && (after_parameters.starts_with('.') || after_parameters.starts_with("->"))
}

/// The ASCII operator that `c`, a typographic character, stands for.
fn ascii_spelling(c: char) -> Option<&'static str> {
    let ascii = match c {
        '\u{2260}' => "!=",
        '\u{2264}' => "<=",
        '\u{2265}' => ">=",
        '\u{2192}' => "->",
        '\u{21d2}' => "=>",
        '\u{00d7}' => "*",
        '\u{00f7}' => "/",
        // The minus sign, en dash, and em dash
        '\u{2212}' | '\u{2013}' | '\u{2014}' => "-",
        '\u{2026}' => "...",
        _ => return None,
    };
    Some(ascii)
}

/// Whether `text` holds a `:` outside the brackets it opens, before it closes one it
/// did not open: in `? b : c)`, but not in `? and {a: 1}`.
fn colon_at_this_level(text: &str) -> bool {
    let mut depth = 0usize;
    for c in text.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => match depth.checked_sub(1) {
                Some(outer) => depth = outer,
                None => return false,
            },
            ':' if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

/// A character as an error names it: a backtick in words, which in backticks would
/// read as noise, and any other in backticks, with its code point when it is not
/// plain visible ASCII.
fn describe_char(c: char) -> String {
    if c == '`' {
        "backtick".to_owned()
    } else if c.is_ascii_graphic() {
        format!("character `{c}`")
    } else if c.is_whitespace() || c.is_control() || c == '\u{feff}' {
        format!("character {}", code_point(c))
    } else {
        format!("character `{c}` ({})", code_point(c))
    }
}

/// The code point of `c`, as in `U+0009`.
pub(crate) fn code_point(c: char) -> String {
    format!("U+{:04X}", u32::from(c))
}
