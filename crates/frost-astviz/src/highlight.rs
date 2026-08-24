//! Syntax highlighting for the source pane, via the tree-sitter Frost grammar
//! and its `highlights.scm` queries. Produces byte-offset spans `{s, e, c}`
//! where `c` is the scope class (dots replaced with dashes to match the CSS).

use tree_sitter::Language;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};
use tree_sitter_language::LanguageFn;

use crate::walk::Highlight;

unsafe extern "C" {
    fn tree_sitter_frost() -> *const ();
}

const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_frost) };

const HIGHLIGHTS_SCM: &str =
    include_str!("../../../../editor/tree-sitter-frost/queries/highlights.scm");

// Recognized scopes, matching the `@captures` in highlights.scm. The index of
// each name is the highlight id reported by the highlighter.
const NAMES: &[&str] = &[
    "comment",
    "constant.builtin",
    "error",
    "function",
    "keyword",
    "number",
    "operator",
    "property",
    "punctuation.bracket",
    "punctuation.delimiter",
    "punctuation.special",
    "string",
    "string.escape",
    "string.special",
    "type",
    "variable",
    "variable.parameter",
];

pub fn highlights(source: &str) -> Vec<Highlight> {
    let language: Language = LANGUAGE.into();

    let mut config = match HighlightConfiguration::new(language, "frost", HIGHLIGHTS_SCM, "", "") {
        Ok(config) => config,
        Err(_) => return Vec::new(),
    };
    config.configure(NAMES);

    let mut highlighter = Highlighter::new();
    let events = match highlighter.highlight(&config, source.as_bytes(), None, |_| None) {
        Ok(events) => events,
        Err(_) => return Vec::new(),
    };

    let mut out = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for event in events {
        let Ok(event) = event else { return out };
        match event {
            HighlightEvent::HighlightStart(h) => stack.push(h.0),
            HighlightEvent::HighlightEnd => {
                stack.pop();
            }
            HighlightEvent::Source { start, end } => {
                if let Some(&top) = stack.last() {
                    out.push(Highlight {
                        s: start,
                        e: end,
                        c: NAMES[top].replace('.', "-"),
                    });
                }
            }
        }
    }
    out
}
