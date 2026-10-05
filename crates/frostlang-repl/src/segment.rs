//! [`complete_segment`]: where a segment of typed source ends.

use std::ops::Range;

use frostlang_parse::{Token, tokens};

/// `source` as a segment ready to run, or `None` if it needs more lines first.
///
/// Source needs more lines while it leaves a bracket or String open, or ends in
/// a `:`, a `->`, or a line continuation: a `\` with nothing after it on its
/// line but spaces. Each line continuation is removed from the segment; the
/// line break after it stays, since newlines separate statements.
///
/// Anything else is complete, even if it will not compile: those errors are
/// left to the compiler. So is a String that cannot span lines, such as a
/// format String, left open. A bracket, `:`, `->`, or `\` inside a String or
/// comment does not count.
///
/// Source starting with `:` is a [metacommand](crate::Repl#metacommands), not
/// Frost source: it needs more lines only after a line continuation.
///
/// ```
/// use frostlang_repl::complete_segment;
///
/// assert_eq!(complete_segment("f(1,"), None);
/// assert_eq!(complete_segment("f(1,\n2)").as_deref(), Some("f(1,\n2)"));
/// assert_eq!(complete_segment("1 + \\\n2").as_deref(), Some("1 + \n2"));
/// assert_eq!(complete_segment(":disassemble if x:").as_deref(), Some(":disassemble if x:"));
/// ```
pub fn complete_segment(source: &str) -> Option<String> {
    let unfinished = if source.starts_with(':') {
        ends_in_line_continuation(source)
    } else {
        is_unfinished(source)
    };
    (!unfinished).then(|| remove_line_continuations(source))
}

fn ends_in_line_continuation(source: &str) -> bool {
    tokens(source)
        .last()
        .is_some_and(|(token, span)| token.is_err() && is_line_continuation(source, span))
}

fn is_unfinished(source: &str) -> bool {
    let mut open_brackets: i64 = 0;
    let mut last = None;
    for (token, span) in tokens(source) {
        match &token {
            Ok(Token::OpenParen | Token::DollarParen | Token::OpenBracket | Token::OpenBrace) => {
                open_brackets += 1;
            }
            Ok(Token::CloseParen | Token::CloseBracket | Token::CloseBrace) => open_brackets -= 1,
            Err(_) if opens_string(&source[span.clone()]) => return true,
            _ => {}
        }
        last = Some((token, span));
    }
    open_brackets > 0
        || match last {
            Some((Ok(Token::Colon | Token::SlimArrow), _)) => true,
            Some((Err(_), span)) => is_line_continuation(source, span),
            _ => false,
        }
}

fn remove_line_continuations(source: &str) -> String {
    let mut kept = String::with_capacity(source.len());
    let mut from = 0;
    for (token, span) in tokens(source) {
        if token.is_err() && is_line_continuation(source, span.clone()) {
            kept.push_str(&source[from..span.start]);
            from = span.end;
        }
    }
    kept.push_str(&source[from..]);
    kept
}

/// Whether the unlexable bytes at `span` are a line continuation.
fn is_line_continuation(source: &str, span: Range<usize>) -> bool {
    let rest_of_line = source[span.end..].split('\n').next().unwrap_or("");
    &source[span] == "\\" && rest_of_line.trim().is_empty()
}

/// Whether unlexable bytes are the start of a String that continues past the
/// end of the source. Only these kinds of String may span lines.
pub(crate) fn opens_string(text: &str) -> bool {
    text.starts_with(['\'', '"'])
}
