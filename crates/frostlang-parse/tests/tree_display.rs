//! `Program::display_tree`: the tree a person reads to see what source parses
//! to. Every node kind is pinned exactly in the default style, ASCII without
//! color; the other styles are checked against it.
//!
//! Each node's line starts with its location, `line:column`, in a gutter
//! right-aligned to the widest. Leaves sit inline after the node's kind.
//! Children are labeled by field only for kinds with several child fields, a
//! list field drawn as its label's own line.

use std::thread;

use frostlang_parse::ast::{Expr, Program, SourceSpan, Spanned, Statement};
use frostlang_parse::{ParseLimits, parse_program};

use crate::helpers::parse;

/// The tree `source` parses to, in the default style.
fn ascii(source: &str) -> String {
    parse(source).display_tree(source).to_string()
}

/// Assert that `source` draws as `expected`. Both are written as raw
/// literals starting with a line break, for alignment; it is not part of
/// either.
#[track_caller]
fn assert_tree(source: &str, expected: &str) {
    let source = source.strip_prefix('\n').unwrap_or(source);
    let expected = expected
        .strip_prefix('\n')
        .expect("the expected tree starts on its own line");
    let actual = ascii(source);
    assert_eq!(
        actual, expected,
        "the tree of:\n{source}\n\nis:\n{actual}\nbut should be:\n{expected}"
    );
}

/// The first line of the tree of `source`, without its gutter.
fn first_line(source: &str) -> String {
    let tree = ascii(source);
    let line = tree.lines().next().expect("the tree has a line");
    let (_, rest) = line.split_once("  ").expect("a gutter starts the line");
    rest.to_owned()
}

// --- Statements and roots ---

#[test]
fn an_empty_program_draws_nothing() {
    assert_eq!(ascii(""), "");
    assert_eq!(ascii("# just a comment\n"), "");
}

#[test]
fn each_top_level_statement_is_a_root() {
    assert_tree(
        r"
a
b
",
        r"
1:1  Name a
2:1  Name b
",
    );
}

#[test]
fn a_def_labels_its_pattern_and_expression() {
    assert_tree(
        "def x = 1",
        r"
1:1  Def
1:5  |-- pattern: BindingPattern x
1:9  `-- expr: Literal 1
",
    );
}

#[test]
fn an_exported_def_says_so_inline() {
    assert_tree(
        "export def x = 1",
        r"
 1:1  Def export
1:12  |-- pattern: BindingPattern x
1:16  `-- expr: Literal 1
",
    );
}

#[test]
fn defn_draws_as_the_def_of_a_named_lambda() {
    assert_tree(
        "defn f(x) -> x",
        r"
 1:1  Def
 1:6  |-- pattern: BindingPattern f
 1:6  `-- expr: Lambda f(x)
1:14      `-- return_expr: Name x
",
    );
}

// --- Leaves ---

#[test]
fn literals_show_as_frost_source() {
    for (source, shown) in [
        ("null", "Literal null"),
        ("true", "Literal true"),
        ("false", "Literal false"),
        ("42", "Literal 42"),
        ("0x1F", "Literal 31"),
        ("1.0", "Literal 1.0"),
        ("2.5", "Literal 2.5"),
        ("1.5e300", "Literal 1.5e300"),
        ("'hi'", r#"Literal "hi""#),
        (r#""hi""#, r#"Literal "hi""#),
        ("''", r#"Literal """#),
        ("x'00ff'", "Literal x'00ff'"),
        ("x''", "Literal x''"),
    ] {
        assert_eq!(first_line(source), shown, "the leaf of `{source}`");
    }
}

#[test]
fn a_string_leaf_escapes_as_frost_displays_a_string() {
    for (source, shown) in [
        (r"'tab\there'", r#"Literal "tab\there""#),
        (r"'a\nb\rc'", r#"Literal "a\nb\rc""#),
        (r#"'say "hi"'"#, r#"Literal "say \"hi\"""#),
        (r"'back\\slash'", r#"Literal "back\\slash""#),
        (r"'\u{1b}\0'", r#"Literal "\u{1b}\u{0}""#),
    ] {
        assert_eq!(first_line(source), shown, "the leaf of `{source}`");
    }
}

#[test]
fn a_name_is_a_leaf() {
    assert_eq!(first_line("foo"), "Name foo");
}

#[test]
fn every_operator_shows_as_its_symbol() {
    for op in ["+", "-", "*", "/", "%", "==", "!=", "<", "<=", ">", ">="] {
        let source = format!("a {op} b");
        assert_eq!(first_line(&source), format!("BinOp {op}"), "`{source}`");
    }
    for op in ["and", "or"] {
        let source = format!("a {op} b");
        assert_eq!(first_line(&source), format!("Logical {op}"), "`{source}`");
    }
    assert_eq!(first_line("-a"), "UnaryOp -");
    assert_eq!(first_line("not a"), "UnaryOp not");
}

// --- Expressions ---

#[test]
fn a_binary_operation_labels_its_operands() {
    assert_tree(
        "1 + 2",
        r"
1:1  BinOp +
1:1  |-- left: Literal 1
1:5  `-- right: Literal 2
",
    );
}

#[test]
fn a_logical_operation_labels_its_operands() {
    assert_tree(
        "a or b",
        r"
1:1  Logical or
1:1  |-- left: Name a
1:6  `-- right: Name b
",
    );
}

#[test]
fn a_unary_operation_has_its_operand_unlabeled() {
    assert_tree(
        "not a",
        r"
1:1  UnaryOp not
1:5  `-- Name a
",
    );
}

#[test]
fn an_if_labels_its_condition_and_branches() {
    assert_tree(
        "if a: 1 else: 2",
        r"
 1:1  If
 1:4  |-- condition: Name a
 1:7  |-- consequent: Literal 1
1:15  `-- alternate: Literal 2
",
    );
}

#[test]
fn an_if_without_else_leaves_out_the_alternate() {
    assert_tree(
        "if a: 1",
        r"
1:1  If
1:4  |-- condition: Name a
1:7  `-- consequent: Literal 1
",
    );
}

#[test]
fn an_elif_is_a_nested_if() {
    assert_tree(
        "if a: 1 elif b: 2 else: 3",
        r"
 1:1  If
 1:4  |-- condition: Name a
 1:7  |-- consequent: Literal 1
 1:9  `-- alternate: If
1:14      |-- condition: Name b
1:17      |-- consequent: Literal 2
1:25      `-- alternate: Literal 3
",
    );
}

#[test]
fn a_do_block_labels_its_body_as_a_list() {
    assert_tree(
        r"
do {
    def x = 1
    print(x)
    x
}
",
        r"
 1:1  Do
      |-- body
 2:5  |   |-- Def
 2:9  |   |   |-- pattern: BindingPattern x
2:13  |   |   `-- expr: Literal 1
 3:5  |   `-- Call
 3:5  |       |-- callee: Name print
      |       `-- args
3:11  |           `-- Name x
 4:5  `-- value: Name x
",
    );
}

#[test]
fn a_do_block_without_statements_leaves_out_its_body() {
    assert_tree(
        "do { x }",
        r"
1:1  Do
1:6  `-- value: Name x
",
    );
}

#[test]
fn a_call_has_a_child_per_argument_under_its_args() {
    assert_tree(
        "f(a, b)",
        r"
1:1  Call
1:1  |-- callee: Name f
     `-- args
1:3      |-- Name a
1:6      `-- Name b
",
    );
}

#[test]
fn a_call_without_arguments_leaves_out_its_args() {
    assert_tree(
        "f()",
        r"
1:1  Call
1:1  `-- callee: Name f
",
    );
}

#[test]
fn a_threaded_call_draws_as_the_call_it_means() {
    assert_tree(
        "a @ f(b)",
        r"
1:1  Call
1:5  |-- callee: Name f
     `-- args
1:1      |-- Name a
1:7      `-- Name b
",
    );
}

#[test]
fn a_soft_index_labels_its_target_and_key() {
    assert_tree(
        "a[0]",
        r"
1:1  SoftIndex
1:1  |-- target: Name a
1:3  `-- key: Literal 0
",
    );
}

#[test]
fn a_hard_index_shows_its_field_inline() {
    assert_tree(
        "a.b.c",
        r"
1:1  HardIndex .c
1:1  `-- HardIndex .b
1:1      `-- Name a
",
    );
}

#[test]
fn an_array_has_its_elements_unlabeled() {
    assert_tree(
        "[1, 2]",
        r"
1:1  Array
1:2  |-- Literal 1
1:5  `-- Literal 2
",
    );
    assert_tree(
        "[]",
        r"
1:1  Array
",
    );
}

#[test]
fn a_map_has_its_entries_unlabeled_and_each_labels_its_key_and_value() {
    assert_tree(
        "{a, b: 2, [3]: 4}",
        r#"
 1:1  Map
 1:2  |-- MapEntry
 1:2  |   |-- key: Literal "a"
 1:2  |   `-- value: Name a
 1:5  |-- MapEntry
 1:5  |   |-- key: Literal "b"
 1:8  |   `-- value: Literal 2
1:11  `-- MapEntry
1:12      |-- key: Literal 3
1:16      `-- value: Literal 4
"#,
    );
    assert_tree(
        "{}",
        r"
1:1  Map
",
    );
}

#[test]
fn a_format_string_has_its_text_and_interpolations_unlabeled() {
    // Text has no span of its own, so no location.
    assert_tree(
        r"$'a${b}c'",
        r#"
1:1  FormatString
     |-- Text "a"
1:6  |-- Name b
     `-- Text "c"
"#,
    );
}

#[test]
fn a_lambda_shows_its_signature_inline() {
    for (source, shown) in [
        ("fn -> 1", "Lambda ()"),
        ("fn x -> 1", "Lambda (x)"),
        ("fn x, _ -> 1", "Lambda (x, _)"),
        ("fn x, ...rest -> 1", "Lambda (x, ...rest)"),
        ("fn ..._ -> 1", "Lambda (..._)"),
        ("fn f(x) -> 1", "Lambda f(x)"),
        ("fn f() -> 1", "Lambda f()"),
    ] {
        assert_eq!(first_line(source), shown, "the leaf of `{source}`");
    }
}

#[test]
fn a_lambda_labels_its_body_and_return_expression() {
    assert_tree(
        r"
fn x -> {
    def y = x
    y
}
",
        r"
 1:1  Lambda (x)
      |-- body
 2:5  |   `-- Def
 2:9  |       |-- pattern: BindingPattern y
2:13  |       `-- expr: Name x
 3:5  `-- return_expr: Name y
",
    );
}

#[test]
fn a_lambda_without_statements_leaves_out_its_body() {
    assert_tree(
        "fn x -> x",
        r"
1:1  Lambda (x)
1:9  `-- return_expr: Name x
",
    );
}

#[test]
fn an_abbreviated_lambda_shows_its_implied_parameters() {
    // An unused positional parameter shows as `_`.
    for (source, shown) in [
        ("$(42)", "AbbreviatedLambda ()"),
        ("$($)", "AbbreviatedLambda ($1)"),
        ("$($1 + $2)", "AbbreviatedLambda ($1, $2)"),
        ("$($3)", "AbbreviatedLambda (_, _, $3)"),
        ("$($$)", "AbbreviatedLambda ($$)"),
        ("$(f($2, $$))", "AbbreviatedLambda (_, $2, $$)"),
    ] {
        assert_eq!(first_line(source), shown, "the leaf of `{source}`");
    }
}

#[test]
fn an_abbreviated_lambda_has_its_body_unlabeled() {
    assert_tree(
        "$($ * 2)",
        r"
1:1  AbbreviatedLambda ($1)
1:3  `-- BinOp *
1:3      |-- left: Name $
1:7      `-- right: Literal 2
",
    );
}

#[test]
fn a_match_labels_its_target_and_arms_and_each_arm_its_parts() {
    assert_tree(
        r"
match v {
    1 => 'one',
    n if: n > 1 => 'more',
}
",
        r#"
 1:1  Match
 1:7  |-- target: Name v
      `-- arms
 2:5      |-- MatchArm
 2:5      |   |-- pattern: ValuePattern
 2:5      |   |   `-- Literal 1
2:10      |   `-- result: Literal "one"
 3:5      `-- MatchArm
 3:5          |-- pattern: BindingPattern n
3:11          |-- guard: BinOp >
3:11          |   |-- left: Name n
3:15          |   `-- right: Literal 1
3:20          `-- result: Literal "more"
"#,
    );
}

// --- Patterns ---

#[test]
fn a_binding_pattern_shows_its_name_and_type_inline() {
    for (source, shown) in [
        ("def x = 1", "pattern: BindingPattern x"),
        ("def _ = 1", "pattern: BindingPattern _"),
        ("def x is Int = 1", "pattern: BindingPattern x is Int"),
        (
            "def _ is Nonnull = 1",
            "pattern: BindingPattern _ is Nonnull",
        ),
    ] {
        let tree = ascii(source);
        assert!(tree.contains(shown), "`{source}` shows `{shown}`:\n{tree}");
    }
}

#[test]
fn a_value_pattern_has_its_expression_unlabeled() {
    assert_tree(
        "match v { (k) => 1 }",
        r"
 1:1  Match
 1:7  |-- target: Name v
      `-- arms
1:11      `-- MatchArm
1:11          |-- pattern: ValuePattern
1:12          |   `-- Name k
1:18          `-- result: Literal 1
",
    );
}

#[test]
fn an_array_pattern_shows_its_rest_and_whole_binding_inline() {
    assert_tree(
        "def [a, ...rest] as all = xs",
        r"
 1:1  Def
 1:5  |-- pattern: ArrayPattern ...rest as all
 1:6  |   `-- BindingPattern a
1:27  `-- expr: Name xs
",
    );
    assert!(
        ascii("def [..._] = xs").contains("pattern: ArrayPattern ..._\n"),
        "a discarded rest shows as `..._`"
    );
    assert!(
        ascii("def [] = xs").contains("pattern: ArrayPattern\n"),
        "an empty Array pattern has nothing inline"
    );
}

#[test]
fn an_array_pattern_shows_a_whole_binding_without_a_rest() {
    assert_tree(
        "def [a] as all = xs",
        r"
 1:1  Def
 1:5  |-- pattern: ArrayPattern as all
 1:6  |   `-- BindingPattern a
1:18  `-- expr: Name xs
",
    );
}

#[test]
fn a_map_pattern_without_a_whole_binding_has_nothing_inline() {
    assert_tree(
        "def {a} = x",
        r#"
 1:1  Def
 1:5  |-- pattern: MapPattern
 1:6  |   `-- MapPatternEntry
 1:6  |       |-- key: Literal "a"
 1:6  |       `-- pattern: BindingPattern a
1:11  `-- expr: Name x
"#,
    );
}

#[test]
fn a_map_pattern_has_its_entries_unlabeled_and_each_labels_its_key_and_pattern() {
    assert_tree(
        "def {a, b?: c, [k]: d} as m = x",
        r#"
 1:1  Def
 1:5  |-- pattern: MapPattern as m
 1:6  |   |-- MapPatternEntry
 1:6  |   |   |-- key: Literal "a"
 1:6  |   |   `-- pattern: BindingPattern a
 1:9  |   |-- MapPatternEntry optional
 1:9  |   |   |-- key: Literal "b"
1:13  |   |   `-- pattern: BindingPattern c
1:16  |   `-- MapPatternEntry
1:17  |       |-- key: Name k
1:21  |       `-- pattern: BindingPattern d
1:31  `-- expr: Name x
"#,
    );
}

#[test]
fn an_alternative_pattern_has_its_alternatives_unlabeled() {
    assert_tree(
        "match v { 1 | [x] => x }",
        r"
 1:1  Match
 1:7  |-- target: Name v
      `-- arms
1:11      `-- MatchArm
1:11          |-- pattern: AlternativePattern
1:11          |   |-- ValuePattern
1:11          |   |   `-- Literal 1
1:15          |   `-- ArrayPattern
1:16          |       `-- BindingPattern x
1:22          `-- result: Name x
",
    );
}

// --- Locations ---

#[test]
fn the_gutter_aligns_locations_to_the_widest() {
    assert_tree(
        r"
a
b
c
d
e
f
g
h
i
j
",
        r"
 1:1  Name a
 2:1  Name b
 3:1  Name c
 4:1  Name d
 5:1  Name e
 6:1  Name f
 7:1  Name g
 8:1  Name h
 9:1  Name i
10:1  Name j
",
    );
}

#[test]
fn columns_count_bytes_as_debug_info_does() {
    // `é` is two bytes, so `x`, the seventh character, is at column 8.
    assert_tree(
        "'é' + x",
        r#"
1:1  BinOp +
1:1  |-- left: Literal "\u{e9}"
1:8  `-- right: Name x
"#,
    );
}

#[test]
fn a_crlf_source_draws_as_its_lf_twin() {
    let lf = "def x = [\n    1,\n]\nx\n";
    let crlf = lf.replace('\n', "\r\n");
    assert_eq!(ascii(&crlf), ascii(lf));
}

#[test]
fn the_wrong_source_gives_wrong_locations_without_panicking() {
    let program = parse("aaaa + bbbb");
    let tree = program.display_tree("").to_string();
    // An offset past the end is placed as if the last line went on.
    let expected = r"
1:1  BinOp +
1:1  |-- left: Name aaaa
1:8  `-- right: Name bbbb
";
    assert_eq!(tree, expected.strip_prefix('\n').unwrap());
}

// --- Truncation ---

/// The longest a leaf shows before it is cut short.
const MAX_LEAF_CHARS: usize = 40;

#[test]
fn a_leaf_at_the_limit_shows_in_full() {
    let text = "a".repeat(MAX_LEAF_CHARS);
    assert_eq!(
        first_line(&format!("'{text}'")),
        format!(r#"Literal "{text}""#)
    );
    assert_eq!(first_line(&text), format!("Name {text}"));
}

#[test]
fn a_leaf_past_the_limit_is_cut_short_with_an_ellipsis() {
    let shown = "a".repeat(MAX_LEAF_CHARS);
    assert_eq!(
        first_line(&format!("'{shown}b'")),
        format!(r#"Literal "{shown}...""#),
        "the ellipsis is inside a String's quotes"
    );
    assert_eq!(first_line(&format!("{shown}b")), format!("Name {shown}..."));
}

#[test]
fn a_bytes_leaf_is_cut_between_bytes() {
    // Each byte shows as two hex digits.
    let shown = "ab".repeat(MAX_LEAF_CHARS / 2);
    assert_eq!(
        first_line(&format!("x'{shown}cd'")),
        format!("Literal x'{shown}...'")
    );
}

#[test]
fn a_cut_never_splits_an_escape() {
    // 20 `\n` escapes fill the limit; the 21st is left out whole.
    let shown = r"\n".repeat(MAX_LEAF_CHARS / 2);
    assert_eq!(
        first_line(&format!("'{shown}\\n'")),
        format!(r#"Literal "{shown}...""#)
    );
    // `\u{1b}` is 6 characters: 6 of them fit, a 7th would pass the limit.
    let shown = r"\u{1b}".repeat(6);
    assert_eq!(
        first_line(&format!("'{shown}\\u{{1b}}'")),
        format!(r#"Literal "{shown}...""#)
    );
}

#[test]
fn format_string_text_is_cut_short_too() {
    let shown = "a".repeat(MAX_LEAF_CHARS);
    let tree = ascii(&format!("$'{shown}b${{x}}'"));
    assert!(tree.contains(&format!(r#"Text "{shown}...""#)), "{tree}");
}

#[test]
fn a_long_lambda_signature_is_cut_short() {
    let tree = first_line("fn alpha, bravo, charlie, delta, echo, foxtrot -> 1");
    assert_eq!(tree, "Lambda (alpha, bravo, charlie, delta, echo, fox...");
}

// --- Styles ---

/// The tree `source` parses to, drawn as `unicode` and `color` say.
fn styled(source: &str, unicode: bool, color: bool) -> String {
    parse(source)
        .display_tree(source)
        .with_unicode(unicode)
        .with_color(color)
        .to_string()
}

/// `text` without its ANSI color codes.
fn uncolored(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find('\x1b') {
        out.push_str(&rest[..at]);
        let end = rest[at..].find('m').expect("a color code ends in `m`");
        rest = &rest[at + end + 1..];
    }
    out.push_str(rest);
    out
}

/// A source drawing every glyph and every kind of text: labeled and
/// unlabeled children, list fields, middle and last children, nodes with and
/// without locations, Strings, and a cut-short leaf.
const VARIED: &str = r"
def xs = [1, f(a, b)]
match xs {
    [n, ...rest] if: n > 0 => $'n is ${n}',
    _ => 'a String long enough to be cut short at the limit',
}
";

#[test]
fn the_default_style_is_ascii_without_color() {
    let program = parse(VARIED);
    assert_eq!(
        program.display_tree(VARIED).to_string(),
        styled(VARIED, false, false)
    );
}

#[test]
fn ascii_output_is_pure_ascii_even_for_non_ascii_text() {
    let source = r"
def greeting = 'héllo, wörld 𝄞'
def map = {['ключ']: $'naïve ${greeting}'}
'é'.repeat
";
    for text in [VARIED, source] {
        let tree = ascii(text);
        assert!(tree.is_ascii(), "{tree}");
        assert!(!tree.contains('\x1b'), "no color: {tree:?}");
    }
    let tree = ascii(source);
    assert!(
        tree.contains(r#"Literal "h\u{e9}llo, w\u{f6}rld \u{1d11e}""#),
        "non-ASCII text escapes:\n{tree}"
    );
}

#[test]
fn unicode_draws_the_same_tree_with_box_drawing() {
    let expected = ascii(VARIED)
        .replace("|-- ", "\u{251c}\u{2500}\u{2500} ")
        .replace("`-- ", "\u{2514}\u{2500}\u{2500} ")
        .replace("|   ", "\u{2502}   ")
        // Only the cut-short String's ellipsis, not the `...` of `...rest`.
        .replace(r#"at ...""#, "at \u{2026}\"");
    assert_eq!(styled(VARIED, true, false), expected);
}

#[test]
fn unicode_shows_non_ascii_text_as_itself_but_still_escapes_control_characters() {
    let source = r"'héllo 𝄞\t\u{1b}'";
    let tree = styled(source, true, false);
    assert_eq!(tree, "1:1  Literal \"héllo 𝄞\\t\\u{1b}\"\n");
}

#[test]
fn unicode_counts_the_limit_in_characters_not_bytes() {
    // `é` is two bytes: a byte count would cut these at 20 characters.
    let shown = "é".repeat(MAX_LEAF_CHARS);
    assert_eq!(
        styled(&format!("'{shown}'"), true, false),
        format!("1:1  Literal \"{shown}\"\n"),
        "a leaf at the limit shows in full"
    );
    assert_eq!(
        styled(&format!("'{shown}é'"), true, false),
        format!("1:1  Literal \"{shown}\u{2026}\"\n"),
        "a leaf past the limit is cut short"
    );
}

#[test]
fn color_adds_only_escape_codes() {
    for unicode in [false, true] {
        let colored = styled(VARIED, unicode, true);
        assert!(colored.contains('\x1b'), "unicode {unicode}: {colored:?}");
        assert_eq!(
            uncolored(&colored),
            styled(VARIED, unicode, false),
            "unicode {unicode}"
        );
    }
}

#[test]
fn color_paints_kinds_labels_locations_and_branches() {
    let colored = styled("f(1)", false, true);
    let painted = |text: &str| colored.contains(&format!("m{text}\x1b[0m"));
    for text in ["Call", "Name", "callee:", "args", "1:1", "1:3", "`-- "] {
        assert!(painted(text), "`{text}` is colored: {colored:?}");
    }
}

// --- Programs not from the parser ---

/// A program of the single expression `expr`, built by hand with spans
/// covering `0..1`.
fn program_of(expr: Expr) -> Program {
    let span = SourceSpan::from(0..1);
    Program {
        statements: vec![Spanned::new(
            Statement::Expr(Spanned::new(expr, span)),
            span,
        )],
    }
}

#[test]
fn names_a_parser_would_reject_still_draw_escaped() {
    // The parser accepts only ASCII identifiers, but a host can build any name.
    let program = program_of(Expr::NameLookup("caf\u{e9}\n\u{1b}[31m".to_owned()));
    let ascii = program.display_tree("x").to_string();
    assert_eq!(ascii, "1:1  Name caf\\u{e9}\\n\\u{1b}[31m\n");
    let unicode = program.display_tree("x").with_unicode(true).to_string();
    assert_eq!(
        unicode, "1:1  Name caf\u{e9}\\n\\u{1b}[31m\n",
        "Unicode shows non-ASCII as itself but still escapes control characters"
    );
}

// --- Depth ---

/// Rust's default thread stack size, which the default `ParseLimits` suit.
const DEFAULT_STACK: usize = 2 * 1024 * 1024;

#[test]
fn the_deepest_tree_the_default_limits_accept_draws_on_a_default_stack() {
    let depth = ParseLimits::new()
        .max_ast_depth
        .expect("the default limits AST depth")
        .get() as usize;
    // Each link of a chain nests the chain before it one level deeper.
    for (shape, link) in [("operator chain", " + a"), ("field chain", ".a")] {
        let source = format!("a{}", link.repeat(depth - 1));
        let drawn = thread::Builder::new()
            .stack_size(DEFAULT_STACK)
            .spawn(move || {
                let program = parse_program("deep.frst", &source)
                    .unwrap_or_else(|error| panic!("{shape} within the limit parses: {error}"));
                [false, true].map(|unicode| {
                    program
                        .display_tree(&source)
                        .with_unicode(unicode)
                        .with_color(true)
                        .to_string()
                        .lines()
                        .count()
                })
            })
            .expect("the thread starts")
            .join()
            .unwrap_or_else(|_| panic!("drawing the {shape} does not overflow the stack"));
        assert!(
            drawn.iter().all(|&lines| lines >= depth),
            "{shape}: every level is drawn, but gave {drawn:?} lines"
        );
    }
}
