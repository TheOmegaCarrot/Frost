//! How the terminal frontend colors source as it is typed. Classes come from
//! the lexer's tokens, so a bracket inside a String or comment is no bracket.

#[cfg(test)]
mod tests;

use std::ops::Range;

use frost_parse::{Token, tokens};

use crate::segment::opens_string;

/// How to color a stretch of source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Class {
    Keyword,
    Number,
    String,
    Comment,
    /// A bracket at this depth of nesting, counted from 0; a closing bracket
    /// takes its opening bracket's depth.
    Bracket(usize),
    /// A closing bracket that closes nothing.
    UnmatchedBracket,
    /// Source with no color of its own.
    Plain,
}

/// `source` divided into stretches to color, in order, covering all of it.
pub(crate) fn classify(source: &str) -> Vec<(Range<usize>, Class)> {
    let mut classes = Vec::new();
    let mut depth = 0;
    let mut covered = 0;
    for (token, span) in tokens(source) {
        classify_gap(source, covered..span.start, &mut classes);
        let class = match token {
            Ok(token) => match token {
                Token::OpenParen | Token::DollarParen | Token::OpenBracket | Token::OpenBrace => {
                    depth += 1;
                    Class::Bracket(depth - 1)
                }
                Token::CloseParen | Token::CloseBracket | Token::CloseBrace => {
                    if depth == 0 {
                        Class::UnmatchedBracket
                    } else {
                        depth -= 1;
                        Class::Bracket(depth)
                    }
                }
                Token::IntLiteral(_) | Token::FloatLiteral(_) => Class::Number,
                Token::RawStringLiteral(_)
                | Token::SingleQuoteStringLiteral(_)
                | Token::DoubleQuoteStringLiteral(_)
                | Token::MultilineStringLiteral(_)
                | Token::SingleQuoteFormatStringLiteral(_)
                | Token::DoubleQuoteFormatStringLiteral(_)
                | Token::BytesLiteral(_) => Class::String,
                token if is_keyword(&token) => Class::Keyword,
                _ => Class::Plain,
            },
            // A String still being typed.
            Err(_) if opens_string(&source[span.clone()]) => Class::String,
            Err(_) => Class::Plain,
        };
        covered = span.end;
        classes.push((span, class));
    }
    classify_gap(source, covered..source.len(), &mut classes);
    classes
}

/// Classify source between tokens: spaces, and possibly a comment running to
/// the end of the line. The lexer emits newlines, so a gap never spans lines.
fn classify_gap(source: &str, gap: Range<usize>, classes: &mut Vec<(Range<usize>, Class)>) {
    if gap.is_empty() {
        return;
    }
    match source[gap.clone()].find('#') {
        Some(at) => {
            let comment = gap.start + at;
            if comment > gap.start {
                classes.push((gap.start..comment, Class::Plain));
            }
            classes.push((comment..gap.end, Class::Comment));
        }
        None => classes.push((gap, Class::Plain)),
    }
}

fn is_keyword(token: &Token<'_>) -> bool {
    matches!(
        token,
        Token::KwAs
            | Token::KwDef
            | Token::KwDefn
            | Token::KwDo
            | Token::KwElif
            | Token::KwElse
            | Token::KwExport
            | Token::KwFalse
            | Token::KwFilter
            | Token::KwFn
            | Token::KwForeach
            | Token::KwIf
            | Token::KwInit
            | Token::KwIs
            | Token::KwMap
            | Token::KwMatch
            | Token::KwNull
            | Token::KwReduce
            | Token::KwTrue
            | Token::KwWith
            | Token::OpAnd
            | Token::OpOr
            | Token::OpNot
    )
}
