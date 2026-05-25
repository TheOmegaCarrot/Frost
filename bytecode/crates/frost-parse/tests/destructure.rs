use frost_parse::ast::*;
use frost_parse::parse_program;

fn str_key(entry: &MapDestructureEntry) -> &[u8] {
    match &entry.key.kind {
        ExprKind::Literal(Literal::String(s)) => s,
        other => panic!("expected string key, got {other:?}"),
    }
}

fn parse(src: &str) -> Program {
    parse_program("test.frst", src).expect(&format!("failed to parse: {src}"))
}

fn parse_err(src: &str) -> String {
    parse_program("test.frst", src)
        .expect_err(&format!("expected parse error for: {src}"))
        .to_string()
}

/// Extract the destructure from a `def ... = 1` statement.
fn def_destructure(src: &str) -> Destructure {
    let program = parse(src);
    assert_eq!(program.statements.len(), 1);
    match &program.statements[0].kind {
        StatementKind::Def { destructure, .. } => destructure.clone(),
        other => panic!("expected Def, got {other:?}"),
    }
}

// -- Simple bindings --

#[test]
fn simple_binding() {
    let d = def_destructure("def x = 1");
    assert!(matches!(d.kind, DestructureKind::Binding(Binding::Named(ref n)) if n == "x"));
}

#[test]
fn discard_binding() {
    let d = def_destructure("def _ = 1");
    assert!(matches!(d.kind, DestructureKind::Binding(Binding::Discarded)));
}

// -- Array destructuring --

#[test]
fn empty_array() {
    let d = def_destructure("def [] = 1");
    match d.kind {
        DestructureKind::Array { elements, rest } => {
            assert!(elements.is_empty());
            assert!(rest.is_none());
        }
        other => panic!("expected Array, got {other:?}"),
    }
}

#[test]
fn single_element_array() {
    let d = def_destructure("def [a] = 1");
    match d.kind {
        DestructureKind::Array { elements, rest } => {
            assert_eq!(elements.len(), 1);
            assert!(matches!(&elements[0].kind, DestructureKind::Binding(Binding::Named(n)) if n == "a"));
            assert!(rest.is_none());
        }
        other => panic!("expected Array, got {other:?}"),
    }
}

#[test]
fn multiple_elements() {
    let d = def_destructure("def [a, b, c] = 1");
    match d.kind {
        DestructureKind::Array { elements, rest } => {
            assert_eq!(elements.len(), 3);
            assert!(matches!(&elements[0].kind, DestructureKind::Binding(Binding::Named(n)) if n == "a"));
            assert!(matches!(&elements[1].kind, DestructureKind::Binding(Binding::Named(n)) if n == "b"));
            assert!(matches!(&elements[2].kind, DestructureKind::Binding(Binding::Named(n)) if n == "c"));
            assert!(rest.is_none());
        }
        other => panic!("expected Array, got {other:?}"),
    }
}

#[test]
fn trailing_comma() {
    let d = def_destructure("def [a, b,] = 1");
    match d.kind {
        DestructureKind::Array { elements, rest } => {
            assert_eq!(elements.len(), 2);
            assert!(rest.is_none());
        }
        other => panic!("expected Array, got {other:?}"),
    }
}

#[test]
fn rest_binding() {
    let d = def_destructure("def [a, b, ...rest] = 1");
    match d.kind {
        DestructureKind::Array { elements, rest } => {
            assert_eq!(elements.len(), 2);
            assert!(matches!(&rest, Some(Binding::Named(n)) if n == "rest"));
        }
        other => panic!("expected Array, got {other:?}"),
    }
}

#[test]
fn rest_discard() {
    let d = def_destructure("def [a, ..._] = 1");
    match d.kind {
        DestructureKind::Array { elements, rest } => {
            assert_eq!(elements.len(), 1);
            assert!(matches!(rest, Some(Binding::Discarded)));
        }
        other => panic!("expected Array, got {other:?}"),
    }
}

#[test]
fn rest_only() {
    let d = def_destructure("def [...xs] = 1");
    match d.kind {
        DestructureKind::Array { elements, rest } => {
            assert!(elements.is_empty());
            assert!(matches!(&rest, Some(Binding::Named(n)) if n == "xs"));
        }
        other => panic!("expected Array, got {other:?}"),
    }
}

// -- Nested arrays --

#[test]
fn nested_array() {
    let d = def_destructure("def [a, [b, c]] = 1");
    match d.kind {
        DestructureKind::Array { elements, rest } => {
            assert_eq!(elements.len(), 2);
            assert!(matches!(&elements[0].kind, DestructureKind::Binding(Binding::Named(n)) if n == "a"));
            match &elements[1].kind {
                DestructureKind::Array { elements: inner, rest: inner_rest } => {
                    assert_eq!(inner.len(), 2);
                    assert!(matches!(&inner[0].kind, DestructureKind::Binding(Binding::Named(n)) if n == "b"));
                    assert!(matches!(&inner[1].kind, DestructureKind::Binding(Binding::Named(n)) if n == "c"));
                    assert!(inner_rest.is_none());
                }
                other => panic!("expected nested Array, got {other:?}"),
            }
            assert!(rest.is_none());
        }
        other => panic!("expected Array, got {other:?}"),
    }
}

#[test]
fn deeply_nested_array() {
    let d = def_destructure("def [a, [b, c, [d, e]]] = 1");
    match d.kind {
        DestructureKind::Array { elements, .. } => {
            assert_eq!(elements.len(), 2);
            match &elements[1].kind {
                DestructureKind::Array { elements: mid, .. } => {
                    assert_eq!(mid.len(), 3);
                    assert!(matches!(&mid[0].kind, DestructureKind::Binding(Binding::Named(n)) if n == "b"));
                    assert!(matches!(&mid[1].kind, DestructureKind::Binding(Binding::Named(n)) if n == "c"));
                    match &mid[2].kind {
                        DestructureKind::Array { elements: inner, .. } => {
                            assert_eq!(inner.len(), 2);
                            assert!(matches!(&inner[0].kind, DestructureKind::Binding(Binding::Named(n)) if n == "d"));
                            assert!(matches!(&inner[1].kind, DestructureKind::Binding(Binding::Named(n)) if n == "e"));
                        }
                        other => panic!("expected innermost Array, got {other:?}"),
                    }
                }
                other => panic!("expected middle Array, got {other:?}"),
            }
        }
        other => panic!("expected Array, got {other:?}"),
    }
}

#[test]
fn nested_with_rest() {
    let d = def_destructure("def [head, [first, ...tail]] = 1");
    match d.kind {
        DestructureKind::Array { elements, rest } => {
            assert_eq!(elements.len(), 2);
            assert!(matches!(&elements[0].kind, DestructureKind::Binding(Binding::Named(n)) if n == "head"));
            match &elements[1].kind {
                DestructureKind::Array { elements: inner, rest: inner_rest } => {
                    assert_eq!(inner.len(), 1);
                    assert!(matches!(&inner[0].kind, DestructureKind::Binding(Binding::Named(n)) if n == "first"));
                    assert!(matches!(inner_rest, Some(Binding::Named(n)) if n == "tail"));
                }
                other => panic!("expected nested Array, got {other:?}"),
            }
            assert!(rest.is_none());
        }
        other => panic!("expected Array, got {other:?}"),
    }
}

#[test]
fn discard_in_nested() {
    let d = def_destructure("def [_, [_, x]] = 1");
    match d.kind {
        DestructureKind::Array { elements, .. } => {
            assert_eq!(elements.len(), 2);
            assert!(matches!(&elements[0].kind, DestructureKind::Binding(Binding::Discarded)));
            match &elements[1].kind {
                DestructureKind::Array { elements: inner, .. } => {
                    assert_eq!(inner.len(), 2);
                    assert!(matches!(&inner[0].kind, DestructureKind::Binding(Binding::Discarded)));
                    assert!(matches!(&inner[1].kind, DestructureKind::Binding(Binding::Named(n)) if n == "x"));
                }
                other => panic!("expected nested Array, got {other:?}"),
            }
        }
        other => panic!("expected Array, got {other:?}"),
    }
}

// -- Export --

#[test]
fn export_def() {
    let program = parse("export def x = 1");
    assert_eq!(program.statements.len(), 1);
    match &program.statements[0].kind {
        StatementKind::Def { exported, .. } => assert!(exported),
        other => panic!("expected Def, got {other:?}"),
    }
}

#[test]
fn non_export_def() {
    let program = parse("def x = 1");
    assert_eq!(program.statements.len(), 1);
    match &program.statements[0].kind {
        StatementKind::Def { exported, .. } => assert!(!exported),
        other => panic!("expected Def, got {other:?}"),
    }
}

// -- Error cases --

#[test]
fn error_missing_equals() {
    let err = parse_err("def x 1");
    assert!(err.contains("Expected ="), "error was: {err}");
}

#[test]
fn error_unexpected_token_in_array() {
    let err = parse_err("def [a + b] = 1");
    assert!(err.contains("unexpected"), "error was: {err}");
}

#[test]
fn error_eof_mid_array() {
    let err = parse_err("def [a, b");
    assert!(
        err.contains("end of input"),
        "error was: {err}"
    );
}

#[test]
fn error_eof_after_def() {
    let err = parse_err("def");
    assert!(
        err.contains("end of input"),
        "error was: {err}"
    );
}

#[test]
fn error_non_identifier_binding() {
    let err = parse_err("def 42 = 1");
    assert!(err.contains("unexpected"), "error was: {err}");
}

// -- Map destructuring --

#[test]
fn empty_map() {
    let d = def_destructure("def {} = 1");
    match d.kind {
        DestructureKind::Map { entries, bind_whole } => {
            assert!(entries.is_empty());
            assert!(bind_whole.is_none());
        }
        other => panic!("expected Map, got {other:?}"),
    }
}

#[test]
fn map_shorthand() {
    let d = def_destructure("def {foo, bar} = 1");
    match d.kind {
        DestructureKind::Map { entries, bind_whole } => {
            assert_eq!(entries.len(), 2);
            assert_eq!(str_key(&entries[0]), b"foo");
            assert!(matches!(&entries[0].destructure.kind, DestructureKind::Binding(Binding::Named(n)) if n == "foo"));
            assert_eq!(str_key(&entries[1]), b"bar");
            assert!(matches!(&entries[1].destructure.kind, DestructureKind::Binding(Binding::Named(n)) if n == "bar"));
            assert!(bind_whole.is_none());
        }
        other => panic!("expected Map, got {other:?}"),
    }
}

#[test]
fn map_explicit_keys() {
    let d = def_destructure("def {foo: a, bar: b} = 1");
    match d.kind {
        DestructureKind::Map { entries, bind_whole } => {
            assert_eq!(entries.len(), 2);
            assert_eq!(str_key(&entries[0]), b"foo");
            assert!(matches!(&entries[0].destructure.kind, DestructureKind::Binding(Binding::Named(n)) if n == "a"));
            assert_eq!(str_key(&entries[1]), b"bar");
            assert!(matches!(&entries[1].destructure.kind, DestructureKind::Binding(Binding::Named(n)) if n == "b"));
            assert!(bind_whole.is_none());
        }
        other => panic!("expected Map, got {other:?}"),
    }
}

#[test]
fn map_computed_key() {
    let d = def_destructure("def {[42]: x} = 1");
    match d.kind {
        DestructureKind::Map { entries, bind_whole } => {
            assert_eq!(entries.len(), 1);
            assert!(matches!(&entries[0].key.kind, ExprKind::Literal(Literal::Int(42))));
            assert!(matches!(&entries[0].destructure.kind, DestructureKind::Binding(Binding::Named(n)) if n == "x"));
            assert!(bind_whole.is_none());
        }
        other => panic!("expected Map, got {other:?}"),
    }
}

#[test]
fn map_mixed_keys() {
    let d = def_destructure("def {foo, bar: b, [42]: c} = 1");
    match d.kind {
        DestructureKind::Map { entries, .. } => {
            assert_eq!(entries.len(), 3);
            assert_eq!(str_key(&entries[0]), b"foo");
            assert!(matches!(&entries[0].destructure.kind, DestructureKind::Binding(Binding::Named(n)) if n == "foo"));
            assert_eq!(str_key(&entries[1]), b"bar");
            assert!(matches!(&entries[1].destructure.kind, DestructureKind::Binding(Binding::Named(n)) if n == "b"));
            assert!(matches!(&entries[2].key.kind, ExprKind::Literal(Literal::Int(42))));
            assert!(matches!(&entries[2].destructure.kind, DestructureKind::Binding(Binding::Named(n)) if n == "c"));
        }
        other => panic!("expected Map, got {other:?}"),
    }
}

#[test]
fn map_trailing_comma() {
    let d = def_destructure("def {foo, bar,} = 1");
    match d.kind {
        DestructureKind::Map { entries, .. } => {
            assert_eq!(entries.len(), 2);
        }
        other => panic!("expected Map, got {other:?}"),
    }
}

#[test]
fn map_as_binding() {
    let d = def_destructure("def {foo, bar} as whole = 1");
    match d.kind {
        DestructureKind::Map { entries, bind_whole } => {
            assert_eq!(entries.len(), 2);
            assert!(matches!(&bind_whole, Some(Binding::Named(n)) if n == "whole"));
        }
        other => panic!("expected Map, got {other:?}"),
    }
}

#[test]
fn map_as_discard() {
    let d = def_destructure("def {foo} as _ = 1");
    match d.kind {
        DestructureKind::Map { entries, bind_whole } => {
            assert_eq!(entries.len(), 1);
            assert!(matches!(bind_whole, Some(Binding::Discarded)));
        }
        other => panic!("expected Map, got {other:?}"),
    }
}

#[test]
fn map_nested_array_value() {
    let d = def_destructure("def {foo: [a, b]} = 1");
    match d.kind {
        DestructureKind::Map { entries, .. } => {
            assert_eq!(entries.len(), 1);
            assert_eq!(str_key(&entries[0]), b"foo");
            match &entries[0].destructure.kind {
                DestructureKind::Array { elements, rest } => {
                    assert_eq!(elements.len(), 2);
                    assert!(matches!(&elements[0].kind, DestructureKind::Binding(Binding::Named(n)) if n == "a"));
                    assert!(matches!(&elements[1].kind, DestructureKind::Binding(Binding::Named(n)) if n == "b"));
                    assert!(rest.is_none());
                }
                other => panic!("expected nested Array, got {other:?}"),
            }
        }
        other => panic!("expected Map, got {other:?}"),
    }
}

#[test]
fn map_nested_map_value() {
    let d = def_destructure("def {outer: {inner: val}} = 1");
    match d.kind {
        DestructureKind::Map { entries, .. } => {
            assert_eq!(entries.len(), 1);
            assert_eq!(str_key(&entries[0]), b"outer");
            match &entries[0].destructure.kind {
                DestructureKind::Map { entries: inner, .. } => {
                    assert_eq!(inner.len(), 1);
                    assert_eq!(str_key(&inner[0]), b"inner");
                    assert!(matches!(&inner[0].destructure.kind, DestructureKind::Binding(Binding::Named(n)) if n == "val"));
                }
                other => panic!("expected nested Map, got {other:?}"),
            }
        }
        other => panic!("expected Map, got {other:?}"),
    }
}

// -- Map error cases --

#[test]
fn error_map_eof_mid_entry() {
    let err = parse_err("def {foo");
    assert!(err.contains("end of input"), "error was: {err}");
}

#[test]
fn error_map_unexpected_token() {
    let err = parse_err("def {foo + bar} = 1");
    assert!(err.contains("unexpected"), "error was: {err}");
}

#[test]
fn error_map_non_identifier_key() {
    let err = parse_err("def {42: x} = 1");
    assert!(err.contains("unexpected"), "error was: {err}");
}

// -- Nested `as` in array position --

#[test]
fn map_as_nested_in_array() {
    let d = def_destructure("def [{name} as person, ...rest] = 1");
    match d.kind {
        DestructureKind::Array { elements, rest } => {
            assert_eq!(elements.len(), 1);
            match &elements[0].kind {
                DestructureKind::Map { entries, bind_whole } => {
                    assert_eq!(entries.len(), 1);
                    assert_eq!(str_key(&entries[0]), b"name");
                    assert!(matches!(&bind_whole, Some(Binding::Named(n)) if n == "person"));
                }
                other => panic!("expected Map in array element, got {other:?}"),
            }
            assert!(matches!(&rest, Some(Binding::Named(n)) if n == "rest"));
        }
        other => panic!("expected Array, got {other:?}"),
    }
}

// -- Statement separation --

#[test]
fn multiple_statements_newline() {
    let program = parse("def a = 1\ndef b = 1");
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn multiple_statements_semicolon() {
    let program = parse("def a = 1; def b = 1");
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn multiple_statements_mixed() {
    let program = parse("def a = 1\ndef b = 1; def c = 1");
    assert_eq!(program.statements.len(), 3);
}

// -- Map empty with `as` --

#[test]
fn empty_map_with_as() {
    let d = def_destructure("def {} as m = 1");
    match d.kind {
        DestructureKind::Map { entries, bind_whole } => {
            assert!(entries.is_empty());
            assert!(matches!(&bind_whole, Some(Binding::Named(n)) if n == "m"));
        }
        other => panic!("expected Map, got {other:?}"),
    }
}

// -- Additional error cases --

#[test]
fn error_trailing_comma_after_rest() {
    let err = parse_err("def [a, ...rest,] = 1");
    assert!(err.contains("Expected ]") || err.contains("unexpected"), "error was: {err}");
}

#[test]
fn error_rest_not_at_end() {
    let err = parse_err("def [...rest, a] = 1");
    assert!(err.contains("Expected ]") || err.contains("unexpected"), "error was: {err}");
}

#[test]
fn error_empty_rest() {
    let err = parse_err("def [...] = 1");
    assert!(err.contains("unexpected"), "error was: {err}");
}

#[test]
fn error_eof_after_as() {
    let err = parse_err("def {foo} as");
    assert!(err.contains("end of input"), "error was: {err}");
}

#[test]
fn error_eof_after_dots() {
    let err = parse_err("def [a, ...");
    assert!(err.contains("end of input"), "error was: {err}");
}

#[test]
fn error_double_rest() {
    let err = parse_err("def [a, ...b, ...c] = 1");
    assert!(err.contains("Expected ]") || err.contains("unexpected"), "error was: {err}");
}
