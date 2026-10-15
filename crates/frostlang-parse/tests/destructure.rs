use frostlang_parse::ast::*;
use frostlang_parse::parse_program;

fn str_key(entry: &Spanned<MapPatternEntry>) -> &str {
    match &entry.node.key.node {
        Expr::Literal(Literal::String(s)) => s,
        other => panic!("expected string key, got {other:?}"),
    }
}

fn parse(src: &str) -> Program {
    parse_program("test.frst", src).unwrap_or_else(|e| panic!("failed to parse: {src}: {e}"))
}

/// The headline message of the error `src` fails to parse with.
fn parse_err_message(src: &str) -> String {
    parse_program("test.frst", src)
        .expect_err(&format!("expected parse error for: {src}"))
        .message()
        .to_owned()
}

/// Extract the pattern from a single `def ... = ...` statement.
fn def_pattern(src: &str) -> Spanned<Pattern> {
    let program = parse(src);
    assert_eq!(program.statements.len(), 1, "{src}: expected one statement");
    match &program.statements[0].node {
        Statement::Def { pattern, .. } => pattern.clone(),
        other => panic!("{src}: expected Def, got {other:?}"),
    }
}

/// The name of an unconstrained, named binding pattern; panics on anything else.
fn named(p: &Spanned<Pattern>) -> &str {
    match &p.node {
        Pattern::Binding {
            name:
                Spanned {
                    node: Binding::Named(n),
                    ..
                },
            type_constraint: None,
        } => n,
        other => panic!("expected an unconstrained named binding, got {other:?}"),
    }
}

/// Whether `p` is an unconstrained `_` binding pattern.
fn is_discard(p: &Spanned<Pattern>) -> bool {
    matches!(
        &p.node,
        Pattern::Binding {
            name: Spanned {
                node: Binding::Discarded,
                ..
            },
            type_constraint: None,
        }
    )
}

/// The name and type constraint of a constrained, named binding pattern.
fn constrained_name(p: &Spanned<Pattern>) -> (&str, TypeConstraint) {
    match &p.node {
        Pattern::Binding {
            name:
                Spanned {
                    node: Binding::Named(n),
                    ..
                },
            type_constraint: Some(tc),
        } => (n, tc.node),
        other => panic!("expected a constrained named binding, got {other:?}"),
    }
}

/// The elements and rest binding of an Array pattern.
fn array(p: &Spanned<Pattern>) -> (&[Spanned<Pattern>], Option<&Binding>) {
    match &p.node {
        Pattern::Array { elements, rest } => (elements, rest.as_ref().map(|b| &b.node)),
        other => panic!("expected Array, got {other:?}"),
    }
}

/// The entries and `as` binding of a Map pattern.
fn map(p: &Spanned<Pattern>) -> (&[Spanned<MapPatternEntry>], Option<&Binding>) {
    match &p.node {
        Pattern::Map {
            entries,
            bind_whole,
        } => (entries, bind_whole.as_ref().map(|b| &b.node)),
        other => panic!("expected Map, got {other:?}"),
    }
}

/// The expression a Value pattern compares against.
fn value(p: &Spanned<Pattern>) -> &Expr {
    match &p.node {
        Pattern::Value(expr) => &expr.node,
        other => panic!("expected Value, got {other:?}"),
    }
}

/// The alternatives of an Alternative pattern.
fn alternatives(p: &Spanned<Pattern>) -> &[Spanned<Pattern>] {
    match &p.node {
        Pattern::Alternative(alts) => alts,
        other => panic!("expected Alternative, got {other:?}"),
    }
}

fn is_named_rest(rest: Option<&Binding>, expected: &str) -> bool {
    matches!(rest, Some(Binding::Named(n)) if n == expected)
}

// -- Simple bindings --

#[test]
fn simple_binding() {
    let d = def_pattern("def x = 1");
    assert_eq!(named(&d), "x");
}

#[test]
fn discard_binding() {
    let d = def_pattern("def _ = 1");
    assert!(is_discard(&d), "expected an unconstrained `_`, got {d:?}");
}

// -- Array destructuring --

#[test]
fn empty_array() {
    let d = def_pattern("def [] = 1");
    let (elements, rest) = array(&d);
    assert!(elements.is_empty());
    assert!(rest.is_none());
}

#[test]
fn single_element_array() {
    let d = def_pattern("def [a] = 1");
    let (elements, rest) = array(&d);
    assert_eq!(elements.len(), 1);
    assert_eq!(named(&elements[0]), "a");
    assert!(rest.is_none());
}

#[test]
fn multiple_elements() {
    let d = def_pattern("def [a, b, c] = 1");
    let (elements, rest) = array(&d);
    assert_eq!(elements.len(), 3);
    assert_eq!(named(&elements[0]), "a");
    assert_eq!(named(&elements[1]), "b");
    assert_eq!(named(&elements[2]), "c");
    assert!(rest.is_none());
}

#[test]
fn trailing_comma() {
    let d = def_pattern("def [a, b,] = 1");
    let (elements, rest) = array(&d);
    assert_eq!(elements.len(), 2);
    assert!(rest.is_none());
}

#[test]
fn rest_binding() {
    let d = def_pattern("def [a, b, ...rest] = 1");
    let (elements, rest) = array(&d);
    assert_eq!(elements.len(), 2);
    assert!(is_named_rest(rest, "rest"), "rest was {rest:?}");
}

#[test]
fn rest_discard() {
    let d = def_pattern("def [a, ..._] = 1");
    let (elements, rest) = array(&d);
    assert_eq!(elements.len(), 1);
    assert_eq!(rest, Some(&Binding::Discarded));
}

#[test]
fn rest_only() {
    let d = def_pattern("def [...xs] = 1");
    let (elements, rest) = array(&d);
    assert!(elements.is_empty());
    assert!(is_named_rest(rest, "xs"), "rest was {rest:?}");
}

// -- Nested arrays --

#[test]
fn nested_array() {
    let d = def_pattern("def [a, [b, c]] = 1");
    let (elements, rest) = array(&d);
    assert_eq!(elements.len(), 2);
    assert_eq!(named(&elements[0]), "a");
    let (inner, inner_rest) = array(&elements[1]);
    assert_eq!(inner.len(), 2);
    assert_eq!(named(&inner[0]), "b");
    assert_eq!(named(&inner[1]), "c");
    assert!(inner_rest.is_none());
    assert!(rest.is_none());
}

#[test]
fn deeply_nested_array() {
    let d = def_pattern("def [a, [b, c, [d, e]]] = 1");
    let (elements, _) = array(&d);
    assert_eq!(elements.len(), 2);
    let (mid, _) = array(&elements[1]);
    assert_eq!(mid.len(), 3);
    assert_eq!(named(&mid[0]), "b");
    assert_eq!(named(&mid[1]), "c");
    let (inner, _) = array(&mid[2]);
    assert_eq!(inner.len(), 2);
    assert_eq!(named(&inner[0]), "d");
    assert_eq!(named(&inner[1]), "e");
}

#[test]
fn nested_with_rest() {
    let d = def_pattern("def [head, [first, ...tail]] = 1");
    let (elements, rest) = array(&d);
    assert_eq!(elements.len(), 2);
    assert_eq!(named(&elements[0]), "head");
    let (inner, inner_rest) = array(&elements[1]);
    assert_eq!(inner.len(), 1);
    assert_eq!(named(&inner[0]), "first");
    assert!(is_named_rest(inner_rest, "tail"), "rest was {inner_rest:?}");
    assert!(rest.is_none());
}

#[test]
fn discard_in_nested() {
    let d = def_pattern("def [_, [_, x]] = 1");
    let (elements, _) = array(&d);
    assert_eq!(elements.len(), 2);
    assert!(is_discard(&elements[0]));
    let (inner, _) = array(&elements[1]);
    assert_eq!(inner.len(), 2);
    assert!(is_discard(&inner[0]));
    assert_eq!(named(&inner[1]), "x");
}

// -- Export --

#[test]
fn export_def() {
    let program = parse("export def x = 1");
    assert_eq!(program.statements.len(), 1);
    match &program.statements[0].node {
        Statement::Def { exported, .. } => assert!(exported),
        other => panic!("expected Def, got {other:?}"),
    }
}

#[test]
fn non_export_def() {
    let program = parse("def x = 1");
    assert_eq!(program.statements.len(), 1);
    match &program.statements[0].node {
        Statement::Def { exported, .. } => assert!(!exported),
        other => panic!("expected Def, got {other:?}"),
    }
}

// -- Error cases --

#[test]
fn error_missing_equals() {
    let err = parse_program("test.frst", "def x 1").unwrap_err();
    assert_eq!(err.message(), "expected `=`, but found `1`");
}

// -- `def` with parameters --
// `def f(x) -> ...` is a likely slip for `defn f(x) -> ...`; the error says so.

/// The primary label of the error `src` fails with, after checking that error
/// is the `def`-with-parameters one and that it points at the `(`.
fn def_with_params_label(src: &str) -> String {
    let err = parse_program("test.frst", src).expect_err(src);
    assert_eq!(err.message(), "`def` takes no parameters", "{src}: {err}");
    let open = src.find('(').unwrap();
    assert_eq!(
        err.primary_span(),
        SourceSpan {
            start: open,
            end: open + 1
        },
        "{src}: the label belongs on the `(`: {err}"
    );
    err.labels()[0].text.clone()
}

#[test]
fn error_def_with_params_suggests_defn() {
    let label = def_with_params_label("def add(x, y) -> x + y");
    assert!(
        label.contains("`defn add(...) -> ...`"),
        "label was: {label}"
    );
}

#[test]
fn error_exported_def_with_params_suggests_export_defn() {
    let label = def_with_params_label("export def add(x, y) -> x + y");
    assert!(
        label.contains("`export defn add(...) -> ...`"),
        "label was: {label}"
    );
}

#[test]
fn error_def_with_params_in_a_block() {
    let source = r"
        do {
            def add(x) -> x + 1
            add(1)
        }
    ";
    let label = def_with_params_label(source);
    assert!(
        label.contains("`defn add(...) -> ...`"),
        "label was: {label}"
    );
}

// `defn` requires a name, so `def _(x)` is not offered a `defn` suggestion.
#[test]
fn error_def_discard_with_params_is_a_plain_error() {
    let err = parse_program("test.frst", "def _(x) -> x").unwrap_err();
    assert_eq!(err.message(), "expected `=`, but found `(`");
    assert!(!err.to_string().contains("defn"), "error was: {err}");
}

#[test]
fn error_unexpected_token_in_array() {
    assert_eq!(
        parse_err_message("def [a + b] = 1"),
        "expected `,` or `]`, but found `+`"
    );
}

#[test]
fn error_eof_mid_array() {
    assert_eq!(
        parse_err_message("def [a, b"),
        "expected `,` or `]`, but found the end of input"
    );
}

#[test]
fn error_eof_after_def() {
    assert_eq!(
        parse_err_message("def"),
        "expected a pattern, but found the end of input"
    );
}

#[test]
fn error_non_pattern_token_after_def() {
    // `+` cannot start a pattern; the old `def 42 = 1` case is now a valid literal pattern.
    assert_eq!(
        parse_err_message("def + = 1"),
        "expected a pattern, but found `+`"
    );
}

// -- Map destructuring --

#[test]
fn empty_map() {
    let d = def_pattern("def {} = 1");
    let (entries, bind_whole) = map(&d);
    assert!(entries.is_empty());
    assert!(bind_whole.is_none());
}

#[test]
fn map_shorthand() {
    let d = def_pattern("def {foo, bar} = 1");
    let (entries, bind_whole) = map(&d);
    assert_eq!(entries.len(), 2);
    assert_eq!(str_key(&entries[0]), "foo");
    assert_eq!(named(&entries[0].node.pattern), "foo");
    assert_eq!(str_key(&entries[1]), "bar");
    assert_eq!(named(&entries[1].node.pattern), "bar");
    assert!(bind_whole.is_none());
}

#[test]
fn map_explicit_keys() {
    let d = def_pattern("def {foo: a, bar: b} = 1");
    let (entries, bind_whole) = map(&d);
    assert_eq!(entries.len(), 2);
    assert_eq!(str_key(&entries[0]), "foo");
    assert_eq!(named(&entries[0].node.pattern), "a");
    assert_eq!(str_key(&entries[1]), "bar");
    assert_eq!(named(&entries[1].node.pattern), "b");
    assert!(bind_whole.is_none());
}

#[test]
fn map_computed_key() {
    let d = def_pattern("def {[42]: x} = 1");
    let (entries, bind_whole) = map(&d);
    assert_eq!(entries.len(), 1);
    assert!(matches!(
        &entries[0].node.key.node,
        Expr::Literal(Literal::Int(42))
    ));
    assert_eq!(named(&entries[0].node.pattern), "x");
    assert!(bind_whole.is_none());
}

#[test]
fn map_mixed_keys() {
    let d = def_pattern("def {foo, bar: b, [42]: c} = 1");
    let (entries, _) = map(&d);
    assert_eq!(entries.len(), 3);
    assert_eq!(str_key(&entries[0]), "foo");
    assert_eq!(named(&entries[0].node.pattern), "foo");
    assert_eq!(str_key(&entries[1]), "bar");
    assert_eq!(named(&entries[1].node.pattern), "b");
    assert!(matches!(
        &entries[2].node.key.node,
        Expr::Literal(Literal::Int(42))
    ));
    assert_eq!(named(&entries[2].node.pattern), "c");
}

#[test]
fn map_trailing_comma() {
    let d = def_pattern("def {foo, bar,} = 1");
    let (entries, _) = map(&d);
    assert_eq!(entries.len(), 2);
}

#[test]
fn map_as_binding() {
    let d = def_pattern("def {foo, bar} as whole = 1");
    let (entries, bind_whole) = map(&d);
    assert_eq!(entries.len(), 2);
    assert!(is_named_rest(bind_whole, "whole"), "as was {bind_whole:?}");
}

#[test]
fn map_as_discard() {
    let d = def_pattern("def {foo} as _ = 1");
    let (entries, bind_whole) = map(&d);
    assert_eq!(entries.len(), 1);
    assert_eq!(bind_whole, Some(&Binding::Discarded));
}

#[test]
fn map_nested_array_value() {
    let d = def_pattern("def {foo: [a, b]} = 1");
    let (entries, _) = map(&d);
    assert_eq!(entries.len(), 1);
    assert_eq!(str_key(&entries[0]), "foo");
    let (elements, rest) = array(&entries[0].node.pattern);
    assert_eq!(elements.len(), 2);
    assert_eq!(named(&elements[0]), "a");
    assert_eq!(named(&elements[1]), "b");
    assert!(rest.is_none());
}

#[test]
fn map_nested_map_value() {
    let d = def_pattern("def {outer: {inner: val}} = 1");
    let (entries, _) = map(&d);
    assert_eq!(entries.len(), 1);
    assert_eq!(str_key(&entries[0]), "outer");
    let (inner, _) = map(&entries[0].node.pattern);
    assert_eq!(inner.len(), 1);
    assert_eq!(str_key(&inner[0]), "inner");
    assert_eq!(named(&inner[0].node.pattern), "val");
}

// -- Map error cases --

#[test]
fn error_map_eof_mid_entry() {
    assert_eq!(
        parse_err_message("def {foo"),
        "expected `:`, `,`, or `}`, but found the end of input"
    );
}

#[test]
fn error_map_unexpected_token() {
    assert_eq!(
        parse_err_message("def {foo + bar} = 1"),
        "expected `,` or `}`, but found `+`"
    );
}

#[test]
fn error_map_non_identifier_key() {
    assert_eq!(
        parse_err_message("def {42: x} = 1"),
        "expected a name or `[`, but found `42`"
    );
}

// -- Nested `as` in array position --

#[test]
fn map_as_nested_in_array() {
    let d = def_pattern("def [{name} as person, ...rest] = 1");
    let (elements, rest) = array(&d);
    assert_eq!(elements.len(), 1);
    let (entries, bind_whole) = map(&elements[0]);
    assert_eq!(entries.len(), 1);
    assert_eq!(str_key(&entries[0]), "name");
    assert!(is_named_rest(bind_whole, "person"), "as was {bind_whole:?}");
    assert!(is_named_rest(rest, "rest"), "rest was {rest:?}");
}

// -- Statement separation --

#[test]
fn multiple_statements_newline() {
    let program = parse(
        r"
def a = 1
def b = 1
",
    );
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn multiple_statements_semicolon() {
    let program = parse("def a = 1; def b = 1");
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn multiple_statements_mixed() {
    let program = parse(
        r"
def a = 1
def b = 1; def c = 1
",
    );
    assert_eq!(program.statements.len(), 3);
}

// A separator with no statement before it is an empty statement, not an error.
// Leading, doubled, and lone separators are all absorbed.

#[test]
fn lone_semicolon_is_empty_program() {
    let program = parse(";");
    assert_eq!(program.statements.len(), 0);
}

#[test]
fn leading_semicolon() {
    let program = parse(";1");
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn doubled_semicolon() {
    let program = parse("1;;2");
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn leading_semicolon_in_scope() {
    let expr = parse("do { ;1 }").statements[0].clone();
    let Statement::Expr(expr) = expr.node else {
        panic!("expected expression statement");
    };
    let Expr::Do { body, value } = expr.node else {
        panic!("expected do block, got {:?}", expr.node);
    };
    assert_eq!(body.len(), 0);
    assert!(matches!(&value.node, Expr::Literal(Literal::Int(1))));
}

// -- Map empty with `as` --

#[test]
fn empty_map_with_as() {
    let d = def_pattern("def {} as m = 1");
    let (entries, bind_whole) = map(&d);
    assert!(entries.is_empty());
    assert!(is_named_rest(bind_whole, "m"), "as was {bind_whole:?}");
}

// -- Additional error cases --

#[test]
fn error_trailing_comma_after_rest() {
    assert_eq!(
        parse_err_message("def [a, ...rest,] = 1"),
        "expected `]`, but found `,`"
    );
}

#[test]
fn error_rest_not_at_end() {
    assert_eq!(
        parse_err_message("def [...rest, a] = 1"),
        "expected `]`, but found `,`"
    );
}

#[test]
fn error_empty_rest() {
    assert_eq!(
        parse_err_message("def [...] = 1"),
        "expected a name after `...`, but found `]`"
    );
}

#[test]
fn error_eof_after_as() {
    assert_eq!(
        parse_err_message("def {foo} as"),
        "expected a name after `as`, but found the end of input"
    );
}

#[test]
fn error_eof_after_dots() {
    assert_eq!(
        parse_err_message("def [a, ..."),
        "expected a name after `...`, but found the end of input"
    );
}

#[test]
fn error_double_rest() {
    assert_eq!(
        parse_err_message("def [a, ...b, ...c] = 1"),
        "expected `]`, but found `,`"
    );
}

// -- Newlines inside destructuring delimiters --
// Newlines are insignificant inside `[...]` / `{...}` destructuring, exactly as
// in array/map literals, call parens, and match patterns. Each test pins one
// newline position, so a partial fix fails precisely.

fn array_names(d: &Spanned<Pattern>) -> Vec<&str> {
    let (elements, _) = array(d);
    elements.iter().map(named).collect()
}

#[test]
fn array_newline_after_open_bracket() {
    let d = def_pattern(
        r"def [
  a, b] = 1",
    );
    assert_eq!(array_names(&d), ["a", "b"]);
}

#[test]
fn array_newline_after_comma() {
    let d = def_pattern(
        r"def [a,
  b] = 1",
    );
    assert_eq!(array_names(&d), ["a", "b"]);
}

#[test]
fn array_newline_before_close_bracket() {
    let d = def_pattern(
        r"def [a, b
] = 1",
    );
    assert_eq!(array_names(&d), ["a", "b"]);
}

#[test]
fn array_trailing_comma_then_newline() {
    let d = def_pattern(
        r"def [
  a,
  b,
] = 1",
    );
    assert_eq!(array_names(&d), ["a", "b"]);
}

#[test]
fn array_blank_line_between_elements() {
    let d = def_pattern(
        r"def [
  a,

  b
] = 1",
    );
    assert_eq!(array_names(&d), ["a", "b"]);
}

#[test]
fn array_rest_on_its_own_line() {
    let d = def_pattern(
        r"def [a,
  ...rest
] = 1",
    );
    let (elements, rest) = array(&d);
    assert_eq!(elements.len(), 1);
    assert!(is_named_rest(rest, "rest"), "rest was {rest:?}");
}

#[test]
fn empty_array_with_newline() {
    let d = def_pattern(
        r"def [
] = 1",
    );
    let (elements, rest) = array(&d);
    assert!(elements.is_empty());
    assert!(rest.is_none());
}

#[test]
fn map_newline_after_open_brace() {
    let d = def_pattern(
        r"def {
  x, y} = 1",
    );
    assert_eq!(map(&d).0.len(), 2);
}

#[test]
fn map_newline_after_comma() {
    let d = def_pattern(
        r"def {x,
  y} = 1",
    );
    assert_eq!(map(&d).0.len(), 2);
}

#[test]
fn map_newline_before_close_brace() {
    let d = def_pattern(
        r"def {x, y
} = 1",
    );
    assert_eq!(map(&d).0.len(), 2);
}

#[test]
fn map_trailing_comma_then_newline() {
    let d = def_pattern(
        r"def {
  x,
  y,
} = 1",
    );
    assert_eq!(map(&d).0.len(), 2);
}

#[test]
fn map_computed_key_on_its_own_line() {
    let d = def_pattern(
        r"def {
  ['k']: a
} = 1",
    );
    let (entries, _) = map(&d);
    assert_eq!(entries.len(), 1);
    assert_eq!(str_key(&entries[0]), "k");
}

#[test]
fn map_entry_newline_after_colon() {
    let d = def_pattern(
        r"def {x:
  [a]} = 1",
    );
    let (entries, _) = map(&d);
    assert_eq!(entries.len(), 1);
    assert!(matches!(
        &entries[0].node.pattern.node,
        Pattern::Array { .. }
    ));
}

#[test]
fn map_computed_key_across_lines() {
    let d = def_pattern(
        r"def {[
'k'
]: a} = 1",
    );
    let (entries, _) = map(&d);
    assert_eq!(entries.len(), 1);
    assert_eq!(str_key(&entries[0]), "k");
}

#[test]
fn map_as_binding_after_multiline_braces() {
    let d = def_pattern(
        r"def {x
} as whole = 1",
    );
    let (_, bind_whole) = map(&d);
    assert!(is_named_rest(bind_whole, "whole"), "as was {bind_whole:?}");
}

#[test]
fn nested_destructures_across_lines() {
    let d = def_pattern(
        r"def [
  {x},
  [a,
   b]
] = 1",
    );
    let (elements, _) = array(&d);
    assert_eq!(elements.len(), 2);
    assert!(matches!(&elements[0].node, Pattern::Map { .. }));
    assert!(matches!(&elements[1].node, Pattern::Array { .. }));
}

// ============================================================
// Full patterns on the left of `def`
// ============================================================
// `def` takes the same patterns as a `match` arm: literals, pins, type
// constraints, and alternatives. The parser is grammar-only, so a `def` that
// binds no names still parses; the compiler rejects it.

/// The first element of the Array pattern in `src`, which must be a Value
/// pattern; returns its expression.
fn first_element_value(src: &str) -> Expr {
    let d = def_pattern(src);
    let (elements, _) = array(&d);
    value(&elements[0]).clone()
}

/// The Value pattern expression under key `k` of the Map pattern in `src`.
fn map_entry_value(src: &str) -> Expr {
    let d = def_pattern(src);
    let (entries, _) = map(&d);
    assert_eq!(str_key(&entries[0]), "k", "{src}: unexpected key");
    value(&entries[0].node.pattern).clone()
}

// -- Literals in an Array pattern --

#[test]
fn array_pattern_int_literal() {
    assert_eq!(
        first_element_value("def [42, x] = v"),
        Expr::Literal(Literal::Int(42))
    );
}

#[test]
fn array_pattern_float_literal() {
    assert_eq!(
        first_element_value("def [2.5, x] = v"),
        Expr::Literal(Literal::Float(2.5))
    );
}

#[test]
fn array_pattern_negative_int_literal() {
    assert_eq!(
        first_element_value("def [-7, x] = v"),
        Expr::Literal(Literal::Int(-7))
    );
}

#[test]
fn array_pattern_negative_float_literal() {
    assert_eq!(
        first_element_value("def [-0.5, x] = v"),
        Expr::Literal(Literal::Float(-0.5))
    );
}

#[test]
fn array_pattern_string_literal() {
    assert_eq!(
        first_element_value("def ['tag', x] = v"),
        Expr::Literal(Literal::String("tag".to_owned()))
    );
}

#[test]
fn array_pattern_raw_string_literal() {
    assert_eq!(
        first_element_value(r"def [R'(a\nb)', x] = v"),
        Expr::Literal(Literal::String(r"a\nb".to_owned())),
        "a raw String keeps its backslashes"
    );
}

#[test]
fn array_pattern_bytes_literal() {
    assert_eq!(
        first_element_value("def [x'6869', x] = v"),
        Expr::Literal(Literal::Bytes(vec![0x68, 0x69]))
    );
}

#[test]
fn array_pattern_format_string() {
    let expr = first_element_value("def [$'id-${n}', x] = v");
    assert!(
        matches!(&expr, Expr::FormatString(segments) if segments.len() == 2),
        "expected a two-segment format String, got {expr:?}"
    );
}

#[test]
fn array_pattern_null_literal() {
    assert_eq!(
        first_element_value("def [null, x] = v"),
        Expr::Literal(Literal::Null)
    );
}

#[test]
fn array_pattern_true_literal() {
    assert_eq!(
        first_element_value("def [true, x] = v"),
        Expr::Literal(Literal::Bool(true))
    );
}

#[test]
fn array_pattern_false_literal() {
    assert_eq!(
        first_element_value("def [false, x] = v"),
        Expr::Literal(Literal::Bool(false))
    );
}

#[test]
fn array_pattern_literal_beside_binding_keeps_binding() {
    let d = def_pattern("def [1, x] = v");
    let (elements, _) = array(&d);
    assert_eq!(elements.len(), 2);
    assert_eq!(
        named(&elements[1]),
        "x",
        "the binding after a literal survives"
    );
}

// -- Literals in a Map pattern --

#[test]
fn map_pattern_int_literal() {
    assert_eq!(
        map_entry_value("def {k: 0} = v"),
        Expr::Literal(Literal::Int(0))
    );
}

#[test]
fn map_pattern_float_literal() {
    assert_eq!(
        map_entry_value("def {k: 1.5} = v"),
        Expr::Literal(Literal::Float(1.5))
    );
}

#[test]
fn map_pattern_negative_literal() {
    assert_eq!(
        map_entry_value("def {k: -3} = v"),
        Expr::Literal(Literal::Int(-3))
    );
}

#[test]
fn map_pattern_string_literal() {
    assert_eq!(
        map_entry_value("def {k: 'point'} = v"),
        Expr::Literal(Literal::String("point".to_owned()))
    );
}

#[test]
fn map_pattern_raw_string_literal() {
    assert_eq!(
        map_entry_value(r"def {k: R'(a\b)'} = v"),
        Expr::Literal(Literal::String(r"a\b".to_owned()))
    );
}

#[test]
fn map_pattern_bytes_literal() {
    assert_eq!(
        map_entry_value("def {k: x'00ff'} = v"),
        Expr::Literal(Literal::Bytes(vec![0x00, 0xff]))
    );
}

#[test]
fn map_pattern_format_string() {
    let expr = map_entry_value("def {k: $'a${b}'} = v");
    assert!(
        matches!(&expr, Expr::FormatString(_)),
        "expected a format String, got {expr:?}"
    );
}

#[test]
fn map_pattern_null_true_false_literals() {
    assert_eq!(
        map_entry_value("def {k: null} = v"),
        Expr::Literal(Literal::Null)
    );
    assert_eq!(
        map_entry_value("def {k: true} = v"),
        Expr::Literal(Literal::Bool(true))
    );
    assert_eq!(
        map_entry_value("def {k: false} = v"),
        Expr::Literal(Literal::Bool(false))
    );
}

#[test]
fn map_pattern_literal_beside_shorthand() {
    let d = def_pattern("def {kind: 'point', x, y} = v");
    let (entries, _) = map(&d);
    assert_eq!(entries.len(), 3);
    assert_eq!(str_key(&entries[0]), "kind");
    assert!(matches!(
        value(&entries[0].node.pattern),
        Expr::Literal(Literal::String(s)) if s == "point"
    ));
    assert_eq!(named(&entries[1].node.pattern), "x");
    assert_eq!(named(&entries[2].node.pattern), "y");
}

// -- Literals and pins at the top level --

#[test]
fn top_level_literal_pattern() {
    let d = def_pattern("def 42 = x");
    assert_eq!(value(&d), &Expr::Literal(Literal::Int(42)));
}

// -- Pins --

#[test]
fn pin_in_array() {
    let d = def_pattern("def [(expected), x] = v");
    let (elements, _) = array(&d);
    assert_eq!(elements.len(), 2);
    assert!(
        matches!(value(&elements[0]), Expr::NameLookup(n) if n == "expected"),
        "expected a pinned name lookup, got {:?}",
        elements[0]
    );
    assert_eq!(named(&elements[1]), "x");
}

#[test]
fn pin_in_map() {
    let d = def_pattern("def {k: (limit)} = v");
    let (entries, _) = map(&d);
    assert!(matches!(
        value(&entries[0].node.pattern),
        Expr::NameLookup(n) if n == "limit"
    ));
}

#[test]
fn pin_with_expression() {
    let d = def_pattern("def [(a + 1)] = v");
    let (elements, _) = array(&d);
    assert!(
        matches!(value(&elements[0]), Expr::BinOp { .. }),
        "a pin holds an arbitrary expression, got {:?}",
        elements[0]
    );
}

#[test]
fn top_level_pin() {
    let d = def_pattern("def (expected) = v");
    assert!(matches!(value(&d), Expr::NameLookup(n) if n == "expected"));
}

// -- Type constraints --

#[test]
fn top_level_type_constraint() {
    let d = def_pattern("def x is Int = 1");
    assert_eq!(constrained_name(&d), ("x", TypeConstraint::Int));
}

#[test]
fn nested_type_constraint() {
    let d = def_pattern("def [a, b is String] = v");
    let (elements, _) = array(&d);
    assert_eq!(named(&elements[0]), "a");
    assert_eq!(
        constrained_name(&elements[1]),
        ("b", TypeConstraint::String)
    );
}

#[test]
fn map_shorthand_type_constraint() {
    let d = def_pattern("def {host is String} = c");
    let (entries, _) = map(&d);
    assert_eq!(entries.len(), 1);
    assert_eq!(str_key(&entries[0]), "host");
    assert_eq!(
        constrained_name(&entries[0].node.pattern),
        ("host", TypeConstraint::String)
    );
}

#[test]
fn map_explicit_key_type_constraint() {
    let d = def_pattern("def {port: p is Int} = c");
    let (entries, _) = map(&d);
    assert_eq!(str_key(&entries[0]), "port");
    assert_eq!(
        constrained_name(&entries[0].node.pattern),
        ("p", TypeConstraint::Int)
    );
}

#[test]
fn discard_with_type_constraint() {
    let d = def_pattern("def _ is Int = 1");
    match &d.node {
        Pattern::Binding {
            name,
            type_constraint: Some(tc),
        } => {
            assert_eq!(name.node, Binding::Discarded);
            assert_eq!(tc.node, TypeConstraint::Int);
        }
        other => panic!("expected a constrained `_`, got {other:?}"),
    }
}

#[test]
fn type_constraint_in_array_discard() {
    let d = def_pattern("def [_ is Int, _ is String] = p");
    let (elements, _) = array(&d);
    assert_eq!(elements.len(), 2);
    for element in elements {
        assert!(
            matches!(
                &element.node,
                Pattern::Binding {
                    name: Spanned {
                        node: Binding::Discarded,
                        ..
                    },
                    type_constraint: Some(_),
                }
            ),
            "expected a constrained `_`, got {element:?}"
        );
    }
}

#[test]
fn exported_def_with_type_constraint() {
    let program = parse("export def x is Int = 1");
    match &program.statements[0].node {
        Statement::Def {
            exported, pattern, ..
        } => {
            assert!(exported);
            assert_eq!(constrained_name(pattern), ("x", TypeConstraint::Int));
        }
        other => panic!("expected Def, got {other:?}"),
    }
}

// -- Alternatives --

#[test]
fn top_level_alternative() {
    let d = def_pattern("def 1 | 2 = x");
    let alts = alternatives(&d);
    assert_eq!(alts.len(), 2);
    assert_eq!(value(&alts[0]), &Expr::Literal(Literal::Int(1)));
    assert_eq!(value(&alts[1]), &Expr::Literal(Literal::Int(2)));
}

#[test]
fn top_level_alternative_of_array_and_binding() {
    let d = def_pattern("def [a] | a = v");
    let alts = alternatives(&d);
    assert_eq!(alts.len(), 2);
    let (elements, _) = array(&alts[0]);
    assert_eq!(named(&elements[0]), "a");
    assert_eq!(named(&alts[1]), "a");
}

#[test]
fn nested_alternative_in_map() {
    let d = def_pattern("def {kind: 'a' | 'b', x} = v");
    let (entries, _) = map(&d);
    assert_eq!(entries.len(), 2);
    assert_eq!(str_key(&entries[0]), "kind");
    let alts = alternatives(&entries[0].node.pattern);
    assert_eq!(alts.len(), 2);
    assert_eq!(value(&alts[0]), &Expr::Literal(Literal::String("a".into())));
    assert_eq!(value(&alts[1]), &Expr::Literal(Literal::String("b".into())));
    assert_eq!(named(&entries[1].node.pattern), "x");
}

#[test]
fn nested_alternative_in_array() {
    let d = def_pattern("def [1 | 2 | 3, x] = v");
    let (elements, _) = array(&d);
    assert_eq!(elements.len(), 2);
    assert_eq!(alternatives(&elements[0]).len(), 3);
    assert_eq!(named(&elements[1]), "x");
}

#[test]
fn alternative_after_newline_inside_brackets() {
    let d = def_pattern(
        r"def [
  1
  | 2,
  x
] = v",
    );
    let (elements, _) = array(&d);
    assert_eq!(elements.len(), 2);
    assert_eq!(alternatives(&elements[0]).len(), 2);
    assert_eq!(named(&elements[1]), "x");
}

#[test]
fn alternative_before_newline_inside_brackets() {
    let d = def_pattern(
        r"def {
  kind: 'a' |
        'b',
  x
} = v",
    );
    let (entries, _) = map(&d);
    assert_eq!(entries.len(), 2);
    assert_eq!(alternatives(&entries[0].node.pattern).len(), 2);
}

// -- `as` on a nested Map --

#[test]
fn as_on_nested_map() {
    let d = def_pattern("def {inner: {a} as whole} = v");
    let (entries, outer_whole) = map(&d);
    assert!(outer_whole.is_none());
    let (inner, inner_whole) = map(&entries[0].node.pattern);
    assert_eq!(inner.len(), 1);
    assert_eq!(str_key(&inner[0]), "a");
    assert!(
        is_named_rest(inner_whole, "whole"),
        "as was {inner_whole:?}"
    );
}

#[test]
fn as_on_top_level_map_with_constrained_entry() {
    let d = def_pattern("def {host is String} as config = c");
    let (entries, whole) = map(&d);
    assert_eq!(entries.len(), 1);
    assert!(is_named_rest(whole, "config"), "as was {whole:?}");
}

// -- A `def` that binds no names still parses --
// Rejecting it is the compiler's job, not the grammar's.

#[test]
fn def_binding_nothing_literal() {
    let d = def_pattern("def 1 = x");
    assert_eq!(value(&d), &Expr::Literal(Literal::Int(1)));
}

#[test]
fn def_binding_nothing_discard() {
    let d = def_pattern("def _ = x");
    assert!(is_discard(&d));
}

#[test]
fn def_binding_nothing_array_of_discards() {
    let d = def_pattern("def [_, _] = p");
    let (elements, rest) = array(&d);
    assert_eq!(elements.len(), 2);
    assert!(elements.iter().all(is_discard));
    assert!(rest.is_none());
}

// -- `export def` with a pattern --

#[test]
fn export_def_with_array_pattern() {
    let program = parse("export def [a, b] = pair");
    match &program.statements[0].node {
        Statement::Def {
            exported, pattern, ..
        } => {
            assert!(exported);
            assert_eq!(array_names(pattern), ["a", "b"]);
        }
        other => panic!("expected Def, got {other:?}"),
    }
}

#[test]
fn export_def_with_map_pattern() {
    let program = parse("export def {host is String, port} = c");
    match &program.statements[0].node {
        Statement::Def {
            exported, pattern, ..
        } => {
            assert!(exported);
            assert_eq!(map(pattern).0.len(), 2);
        }
        other => panic!("expected Def, got {other:?}"),
    }
}

// -- Negative cases --

#[test]
fn error_guard_on_def_is_not_allowed() {
    // Guards belong to match arms, not to patterns.
    let message = parse_err_message("def x if: true = 1");
    assert_eq!(message, "expected `=`, but found `if`");
}

#[test]
fn error_newline_before_equals_at_top_level() {
    let src = r"
def x
= 1
";
    let err = parse_program("test.frst", src).expect_err("a newline ends the `def` pattern");
    assert_eq!(err.message(), "expected `=`, but found a line break");
}

#[test]
fn error_missing_equals_after_constrained_name() {
    assert_eq!(
        parse_err_message("def x is Int 1"),
        "expected `=`, but found `1`"
    );
}

// -- The `defn` hint --

#[test]
fn defn_hint_is_not_offered_after_a_type_constraint() {
    let err = parse_program("test.frst", "def f is Int = 1");
    assert!(err.is_ok(), "`def f is Int = 1` is a valid constrained def");
    let err = parse_program("test.frst", "def f is Int(x) = 1").unwrap_err();
    assert_ne!(
        err.message(),
        "`def` takes no parameters",
        "a constrained name is not a `defn` slip: {err}"
    );
}

#[test]
fn defn_hint_still_fires_for_bare_name_with_params() {
    let label = def_with_params_label("def f(x) = 1");
    assert!(label.contains("`defn f(...) -> ...`"), "label was: {label}");
}

// -- Placeholders are rejected in every pattern definition position --

/// The message and help of the error `src` fails to parse with.
fn parse_err_message_and_help(src: &str) -> (String, Option<String>) {
    let err = parse_program("test.frst", src).expect_err(&format!("expected parse error: {src}"));
    (err.message().to_owned(), err.help().map(str::to_owned))
}

#[test]
fn placeholders_are_rejected_in_every_def_pattern_position() {
    const PLACEHOLDER_HELP: &str = "`$` placeholders work only inside `$( ... )`, like `$($ * 2)`";
    for (src, message) in [
        // An element of an Array pattern.
        ("def [$] = x", "expected a pattern, but found `$`"),
        // The value pattern of a Map entry.
        ("def {a: $2} = m", "expected a pattern, but found `$2`"),
        // A branch of an alternative.
        ("def x is Int | $ = y", "expected a pattern, but found `$`"),
        // The rest binding.
        (
            "def [a, ...$] = x",
            "expected a name after `...`, but found `$`",
        ),
        // The whole-Map binding.
        (
            "def {a} as $ = m",
            "expected a name after `as`, but found `$`",
        ),
        // The rest placeholder as an element.
        ("def [a, $$] = x", "expected a pattern, but found `$$`"),
        // A Map shorthand key.
        ("def {$} = m", "expected a name or `[`, but found `$`"),
        // The top-level pattern, before a type constraint.
        ("def $ is Int = x", "expected a pattern, but found `$`"),
    ] {
        let (actual_message, help) = parse_err_message_and_help(src);
        assert_eq!(actual_message, message, "{src}: message");
        assert_eq!(help.as_deref(), Some(PLACEHOLDER_HELP), "{src}: help");
    }
}

// -- Pattern spans --

/// The `(start, end)` source offsets of a pattern.
fn span_of(p: &Spanned<Pattern>) -> (usize, usize) {
    (p.span.start, p.span.end)
}

#[test]
fn a_pin_spans_its_parentheses_but_its_expression_does_not() {
    let src = "def (a) = x";
    let pin = def_pattern(src);
    assert_eq!(span_of(&pin), (4, 7), "the pin includes `(` and `)`");
    assert_eq!(&src[4..7], "(a)");
    let inner = match &pin.node {
        Pattern::Value(expr) => expr,
        other => panic!("expected a pin, got {other:?}"),
    };
    assert_eq!(
        (inner.span.start, inner.span.end),
        (5, 6),
        "the inner expression excludes the parentheses"
    );
}

#[test]
fn a_pin_nested_in_an_array_keeps_its_parentheses_in_its_span() {
    let src = "def [(a)] = x";
    let pattern = def_pattern(src);
    let (elements, _) = array(&pattern);
    assert_eq!(span_of(&elements[0]), (5, 8));
    assert_eq!(span_of(&pattern), (4, 9));
}

#[test]
fn a_constrained_binding_ends_at_its_type() {
    let src = "def x is Int = 1";
    let pattern = def_pattern(src);
    assert_eq!(span_of(&pattern), (4, 12));
    assert_eq!(&src[4..12], "x is Int");
}

#[test]
fn a_map_pattern_with_a_whole_binding_ends_at_the_binding() {
    let src = "def {a} as m = q";
    let pattern = def_pattern(src);
    assert_eq!(span_of(&pattern), (4, 12));
    assert_eq!(&src[4..12], "{a} as m");
}

#[test]
fn an_alternative_spans_from_its_first_branch_to_its_last() {
    let src = "def 1 | 2 | 3 = x";
    let pattern = def_pattern(src);
    assert_eq!(span_of(&pattern), (4, 13));
    assert_eq!(&src[4..13], "1 | 2 | 3");
    let branches: Vec<_> = alternatives(&pattern).iter().map(span_of).collect();
    assert_eq!(branches, [(4, 5), (8, 9), (12, 13)]);
}

#[test]
fn an_alternative_with_a_pin_spans_the_pin_parentheses() {
    let src = "def (a) | 1 = x";
    let pattern = def_pattern(src);
    assert_eq!(span_of(&pattern), (4, 11));
    assert_eq!(span_of(&alternatives(&pattern)[0]), (4, 7));
}

#[test]
fn a_negative_literal_pattern_starts_at_its_minus_sign() {
    let src = "def -1 = x";
    let pattern = def_pattern(src);
    assert_eq!(span_of(&pattern), (4, 6));
    assert_eq!(&src[4..6], "-1");
}

#[test]
fn a_constrained_map_shorthand_entry_spans_name_through_type() {
    let src = "def {host is String} = m";
    let pattern = def_pattern(src);
    assert_eq!(span_of(&pattern), (4, 20));
    let (entries, _) = map(&pattern);
    let entry = &entries[0];
    assert_eq!((entry.span.start, entry.span.end), (5, 19));
    assert_eq!(&src[5..19], "host is String");
    assert_eq!(span_of(&entry.node.pattern), (5, 19));
    assert_eq!(
        (entry.node.key.span.start, entry.node.key.span.end),
        (5, 9),
        "the key is the name alone"
    );
}
