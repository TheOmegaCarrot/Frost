mod helpers;

use frost_parse::ast::*;
use helpers::*;

fn assert_match(expr: &Expr) -> (&Expr, &[MatchArm]) {
    match &expr.kind {
        ExprKind::Match { target, arms } => (target, arms),
        other => panic!("expected Match, got {other:?}"),
    }
}

fn assert_binding(pat: &MatchPattern) -> (&Binding, Option<TypeConstraint>) {
    match &pat.kind {
        MatchPatternKind::Binding {
            name,
            type_constraint,
        } => (name, *type_constraint),
        other => panic!("expected Binding pattern, got {other:?}"),
    }
}

fn assert_value_pattern(pat: &MatchPattern) -> &Expr {
    match &pat.kind {
        MatchPatternKind::Value(expr) => expr,
        other => panic!("expected Value pattern, got {other:?}"),
    }
}

fn assert_array_pattern(pat: &MatchPattern) -> (&[MatchPattern], Option<&Binding>) {
    match &pat.kind {
        MatchPatternKind::Array { elements, rest } => (elements, rest.as_ref()),
        other => panic!("expected Array pattern, got {other:?}"),
    }
}

fn assert_map_pattern(pat: &MatchPattern) -> (&[MapPatternEntry], Option<&Binding>) {
    match &pat.kind {
        MatchPatternKind::Map {
            entries,
            bind_whole,
        } => (entries, bind_whole.as_ref()),
        other => panic!("expected Map pattern, got {other:?}"),
    }
}

fn assert_alternative(pat: &MatchPattern) -> &[MatchPattern] {
    match &pat.kind {
        MatchPatternKind::Alternative(alts) => alts,
        other => panic!("expected Alternative pattern, got {other:?}"),
    }
}

// ============================================================
// Basic match structure
// ============================================================

mod basic {
    use super::*;

    #[test]
    fn single_arm() {
        let expr = parse_expr("match x { _ => 1 }");
        let (target, arms) = assert_match(&expr);
        assert!(matches!(&target.kind, ExprKind::NameLookup(n) if n == "x"));
        assert_eq!(arms.len(), 1);
        let (binding, tc) = assert_binding(&arms[0].pattern);
        assert_eq!(*binding, Binding::Discarded);
        assert!(tc.is_none());
        assert!(is_int(&arms[0].result, 1));
    }

    #[test]
    fn multiple_arms() {
        let expr = parse_expr("match x { 1 => 'one', 2 => 'two', _ => 'other' }");
        let (_, arms) = assert_match(&expr);
        assert_eq!(arms.len(), 3);
        let v = assert_value_pattern(&arms[0].pattern);
        assert!(is_int(v, 1));
        let v = assert_value_pattern(&arms[1].pattern);
        assert!(is_int(v, 2));
        assert_binding(&arms[2].pattern);
    }

    #[test]
    fn trailing_comma() {
        let expr = parse_expr("match x { 1 => 'one', _ => 'other', }");
        let (_, arms) = assert_match(&expr);
        assert_eq!(arms.len(), 2);
    }

    #[test]
    fn empty_match() {
        let expr = parse_expr("match x {}");
        let (_, arms) = assert_match(&expr);
        assert!(arms.is_empty());
    }

    #[test]
    fn expression_target() {
        let expr = parse_expr("match 1 + 2 { _ => null }");
        let (target, _) = assert_match(&expr);
        assert!(is_binop(target).is_some());
    }

    #[test]
    fn match_in_def() {
        let program = parse("def result = match x { _ => 1 }");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StatementKind::Def { expr, .. } => {
                assert!(matches!(&expr.kind, ExprKind::Match { .. }));
            }
            other => panic!("expected Def, got {other:?}"),
        }
    }
}

// ============================================================
// Literal value patterns
// ============================================================

mod literals {
    use super::*;

    #[test]
    fn int_literal() {
        let expr = parse_expr("match x { 42 => true }");
        let (_, arms) = assert_match(&expr);
        let v = assert_value_pattern(&arms[0].pattern);
        assert!(is_int(v, 42));
    }

    #[test]
    fn float_literal() {
        let expr = parse_expr("match x { 3.14 => true }");
        let (_, arms) = assert_match(&expr);
        let v = assert_value_pattern(&arms[0].pattern);
        assert!(matches!(&v.kind, ExprKind::Literal(Literal::Float(n)) if *n == 3.14));
    }

    #[test]
    fn true_literal() {
        let expr = parse_expr("match x { true => 1 }");
        let (_, arms) = assert_match(&expr);
        let v = assert_value_pattern(&arms[0].pattern);
        assert!(matches!(&v.kind, ExprKind::Literal(Literal::Bool(true))));
    }

    #[test]
    fn false_literal() {
        let expr = parse_expr("match x { false => 1 }");
        let (_, arms) = assert_match(&expr);
        let v = assert_value_pattern(&arms[0].pattern);
        assert!(matches!(&v.kind, ExprKind::Literal(Literal::Bool(false))));
    }

    #[test]
    fn null_literal() {
        let expr = parse_expr("match x { null => 1 }");
        let (_, arms) = assert_match(&expr);
        let v = assert_value_pattern(&arms[0].pattern);
        assert!(matches!(&v.kind, ExprKind::Literal(Literal::Null)));
    }

    #[test]
    fn string_literal() {
        let expr = parse_expr("match x { 'hello' => 1 }");
        let (_, arms) = assert_match(&expr);
        let v = assert_value_pattern(&arms[0].pattern);
        assert!(matches!(&v.kind, ExprKind::Literal(Literal::String(s)) if s == b"hello"));
    }

    #[test]
    fn negative_int_literal() {
        let expr = parse_expr("match x { -1 => true }");
        let (_, arms) = assert_match(&expr);
        let v = assert_value_pattern(&arms[0].pattern);
        assert!(is_int(v, -1));
    }

    #[test]
    fn negative_float_literal() {
        let expr = parse_expr("match x { -3.14 => true }");
        let (_, arms) = assert_match(&expr);
        let v = assert_value_pattern(&arms[0].pattern);
        assert!(matches!(&v.kind, ExprKind::Literal(Literal::Float(n)) if *n == -3.14));
    }

    #[test]
    fn parenthesized_value_comparison() {
        let expr = parse_expr("match x { (some_var) => 1 }");
        let (_, arms) = assert_match(&expr);
        let v = assert_value_pattern(&arms[0].pattern);
        assert!(matches!(&v.kind, ExprKind::NameLookup(n) if n == "some_var"));
    }

    #[test]
    fn parenthesized_expression() {
        let expr = parse_expr("match x { (1 + 2) => true }");
        let (_, arms) = assert_match(&expr);
        let v = assert_value_pattern(&arms[0].pattern);
        assert!(is_binop(v).is_some());
    }
}

// ============================================================
// Binding patterns
// ============================================================

mod bindings {
    use super::*;

    #[test]
    fn named_binding() {
        let expr = parse_expr("match x { n => n }");
        let (_, arms) = assert_match(&expr);
        let (binding, tc) = assert_binding(&arms[0].pattern);
        assert_eq!(*binding, Binding::Named("n".into()));
        assert!(tc.is_none());
    }

    #[test]
    fn discard_binding() {
        let expr = parse_expr("match x { _ => 1 }");
        let (_, arms) = assert_match(&expr);
        let (binding, _) = assert_binding(&arms[0].pattern);
        assert_eq!(*binding, Binding::Discarded);
    }

    #[test]
    fn binding_with_type_constraint() {
        let expr = parse_expr("match x { n is Int => n }");
        let (_, arms) = assert_match(&expr);
        let (binding, tc) = assert_binding(&arms[0].pattern);
        assert_eq!(*binding, Binding::Named("n".into()));
        assert!(matches!(tc, Some(TypeConstraint::Int)));
    }

    #[test]
    fn discard_with_type_constraint() {
        let expr = parse_expr("match x { _ is String => 1 }");
        let (_, arms) = assert_match(&expr);
        let (binding, tc) = assert_binding(&arms[0].pattern);
        assert_eq!(*binding, Binding::Discarded);
        assert!(matches!(tc, Some(TypeConstraint::String)));
    }

    #[test]
    fn all_type_constraints() {
        let types = [
            ("Null", TypeConstraint::Null),
            ("Int", TypeConstraint::Int),
            ("Float", TypeConstraint::Float),
            ("Bool", TypeConstraint::Bool),
            ("String", TypeConstraint::String),
            ("Array", TypeConstraint::Array),
            ("Map", TypeConstraint::Map),
            ("Function", TypeConstraint::Function),
            ("Primitive", TypeConstraint::Primitive),
            ("Numeric", TypeConstraint::Numeric),
            ("Structured", TypeConstraint::Structured),
            ("Nonnull", TypeConstraint::Nonnull),
        ];

        for (name, expected) in types {
            let src = format!("match x {{ _ is {name} => 1 }}");
            let expr = parse_expr(&src);
            let (_, arms) = assert_match(&expr);
            let (_, tc) = assert_binding(&arms[0].pattern);
            assert_eq!(
                std::mem::discriminant(&tc.expect("type constraint")),
                std::mem::discriminant(&expected),
                "failed for {name}"
            );
        }
    }
}

// ============================================================
// Guards
// ============================================================

mod guards {
    use super::*;

    #[test]
    fn simple_guard() {
        let expr = parse_expr("match x { n if: n > 0 => n }");
        let (_, arms) = assert_match(&expr);
        assert!(arms[0].guard.is_some());
        let guard = arms[0].guard.as_ref().expect("guard");
        assert!(is_binop(guard).is_some());
    }

    #[test]
    fn guard_with_type_constraint() {
        let expr = parse_expr("match x { n is Int if: n > 0 => n }");
        let (_, arms) = assert_match(&expr);
        let (_, tc) = assert_binding(&arms[0].pattern);
        assert!(matches!(tc, Some(TypeConstraint::Int)));
        assert!(arms[0].guard.is_some());
    }

    #[test]
    fn no_guard() {
        let expr = parse_expr("match x { n => n }");
        let (_, arms) = assert_match(&expr);
        assert!(arms[0].guard.is_none());
    }

    #[test]
    fn guard_on_discard() {
        let expr = parse_expr("match x { _ if: true => 1 }");
        let (_, arms) = assert_match(&expr);
        assert!(arms[0].guard.is_some());
    }
}

// ============================================================
// Alternatives
// ============================================================

mod alternatives {
    use super::*;

    #[test]
    fn literal_alternatives() {
        let expr = parse_expr("match x { 1 | 2 | 3 => 'low' }");
        let (_, arms) = assert_match(&expr);
        let alts = assert_alternative(&arms[0].pattern);
        assert_eq!(alts.len(), 3);
        assert!(is_int(assert_value_pattern(&alts[0]), 1));
        assert!(is_int(assert_value_pattern(&alts[1]), 2));
        assert!(is_int(assert_value_pattern(&alts[2]), 3));
    }

    #[test]
    fn binding_alternatives() {
        let expr = parse_expr("match x { n is Int | n is Float => n }");
        let (_, arms) = assert_match(&expr);
        let alts = assert_alternative(&arms[0].pattern);
        assert_eq!(alts.len(), 2);
        let (_, tc1) = assert_binding(&alts[0]);
        let (_, tc2) = assert_binding(&alts[1]);
        assert!(matches!(tc1, Some(TypeConstraint::Int)));
        assert!(matches!(tc2, Some(TypeConstraint::Float)));
    }

    #[test]
    fn nested_alternatives_in_array() {
        let expr = parse_expr("match x { [1 | 2, 'a' | 'b'] => true, _ => false }");
        let (_, arms) = assert_match(&expr);
        let (elements, _) = assert_array_pattern(&arms[0].pattern);
        assert_eq!(elements.len(), 2);
        let alts0 = assert_alternative(&elements[0]);
        assert_eq!(alts0.len(), 2);
        let alts1 = assert_alternative(&elements[1]);
        assert_eq!(alts1.len(), 2);
    }

    #[test]
    fn guard_after_alternatives() {
        let expr = parse_expr("match x { n is Int | n is Float if: n > 0 => n }");
        let (_, arms) = assert_match(&expr);
        assert_alternative(&arms[0].pattern);
        assert!(arms[0].guard.is_some());
    }
}

// ============================================================
// Array patterns
// ============================================================

mod array_patterns {
    use super::*;

    #[test]
    fn empty_array() {
        let expr = parse_expr("match x { [] => null }");
        let (_, arms) = assert_match(&expr);
        let (elements, rest) = assert_array_pattern(&arms[0].pattern);
        assert!(elements.is_empty());
        assert!(rest.is_none());
    }

    #[test]
    fn exact_elements() {
        let expr = parse_expr("match x { [a, b, c] => a }");
        let (_, arms) = assert_match(&expr);
        let (elements, rest) = assert_array_pattern(&arms[0].pattern);
        assert_eq!(elements.len(), 3);
        assert!(rest.is_none());
        let (b, _) = assert_binding(&elements[0]);
        assert_eq!(*b, Binding::Named("a".into()));
    }

    #[test]
    fn with_rest() {
        let expr = parse_expr("match x { [head, ...tail] => head }");
        let (_, arms) = assert_match(&expr);
        let (elements, rest) = assert_array_pattern(&arms[0].pattern);
        assert_eq!(elements.len(), 1);
        assert_eq!(*rest.expect("rest binding"), Binding::Named("tail".into()));
    }

    #[test]
    fn with_discard_rest() {
        let expr = parse_expr("match x { [a, b, ..._] => a }");
        let (_, arms) = assert_match(&expr);
        let (elements, rest) = assert_array_pattern(&arms[0].pattern);
        assert_eq!(elements.len(), 2);
        assert_eq!(*rest.expect("rest binding"), Binding::Discarded);
    }

    #[test]
    fn nested_literal_patterns() {
        let expr = parse_expr("match x { [1, 'two', true] => null }");
        let (_, arms) = assert_match(&expr);
        let (elements, _) = assert_array_pattern(&arms[0].pattern);
        assert_eq!(elements.len(), 3);
        assert!(is_int(assert_value_pattern(&elements[0]), 1));
    }

    #[test]
    fn nested_array_in_array() {
        let expr = parse_expr("match x { [[a, b], c] => a }");
        let (_, arms) = assert_match(&expr);
        let (elements, _) = assert_array_pattern(&arms[0].pattern);
        assert_eq!(elements.len(), 2);
        let (inner, _) = assert_array_pattern(&elements[0]);
        assert_eq!(inner.len(), 2);
    }

    #[test]
    fn trailing_comma() {
        let expr = parse_expr("match x { [a, b,] => a }");
        let (_, arms) = assert_match(&expr);
        let (elements, _) = assert_array_pattern(&arms[0].pattern);
        assert_eq!(elements.len(), 2);
    }
}

// ============================================================
// Map patterns
// ============================================================

mod map_patterns {
    use super::*;

    #[test]
    fn simple_map() {
        let expr = parse_expr("match x { {name: n} => n }");
        let (_, arms) = assert_match(&expr);
        let (entries, bind_whole) = assert_map_pattern(&arms[0].pattern);
        assert_eq!(entries.len(), 1);
        assert!(bind_whole.is_none());

        assert!(matches!(&entries[0].key.kind, ExprKind::Literal(Literal::String(s)) if s == b"name"));
        let (b, _) = assert_binding(&entries[0].pattern);
        assert_eq!(*b, Binding::Named("n".into()));
    }

    #[test]
    fn shorthand() {
        let expr = parse_expr("match x { {name} => name }");
        let (_, arms) = assert_match(&expr);
        let (entries, _) = assert_map_pattern(&arms[0].pattern);
        assert_eq!(entries.len(), 1);
        assert!(matches!(&entries[0].key.kind, ExprKind::Literal(Literal::String(s)) if s == b"name"));
        let (b, _) = assert_binding(&entries[0].pattern);
        assert_eq!(*b, Binding::Named("name".into()));
    }

    #[test]
    fn shorthand_with_type_constraint() {
        let expr = parse_expr("match x { {name is String} => name }");
        let (_, arms) = assert_match(&expr);
        let (entries, _) = assert_map_pattern(&arms[0].pattern);
        assert_eq!(entries.len(), 1);
        let (b, tc) = assert_binding(&entries[0].pattern);
        assert_eq!(*b, Binding::Named("name".into()));
        assert!(matches!(tc, Some(TypeConstraint::String)));
    }

    #[test]
    fn value_pattern_in_map() {
        let expr = parse_expr("match x { {role: 'admin'} => true }");
        let (_, arms) = assert_match(&expr);
        let (entries, _) = assert_map_pattern(&arms[0].pattern);
        let v = assert_value_pattern(&entries[0].pattern);
        assert!(matches!(&v.kind, ExprKind::Literal(Literal::String(s)) if s == b"admin"));
    }

    #[test]
    fn as_binding() {
        let expr = parse_expr("match x { {name: n} as person => n }");
        let (_, arms) = assert_match(&expr);
        let (entries, bind_whole) = assert_map_pattern(&arms[0].pattern);
        assert_eq!(entries.len(), 1);
        assert_eq!(
            *bind_whole.expect("as binding"),
            Binding::Named("person".into())
        );
    }

    #[test]
    fn as_discard() {
        let expr = parse_expr("match x { {name: n} as _ => n }");
        let (_, arms) = assert_match(&expr);
        let (_, bind_whole) = assert_map_pattern(&arms[0].pattern);
        assert_eq!(*bind_whole.expect("as binding"), Binding::Discarded);
    }

    #[test]
    fn computed_key() {
        let expr = parse_expr("match x { {[1 + 2]: p} => p }");
        let (_, arms) = assert_match(&expr);
        let (entries, _) = assert_map_pattern(&arms[0].pattern);
        assert!(is_binop(&entries[0].key).is_some());
        let (b, _) = assert_binding(&entries[0].pattern);
        assert_eq!(*b, Binding::Named("p".into()));
    }

    #[test]
    fn multiple_entries() {
        let expr = parse_expr("match x { {name: n, age: a} => n }");
        let (_, arms) = assert_match(&expr);
        let (entries, _) = assert_map_pattern(&arms[0].pattern);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn trailing_comma() {
        let expr = parse_expr("match x { {name: n, age: a,} => n }");
        let (_, arms) = assert_match(&expr);
        let (entries, _) = assert_map_pattern(&arms[0].pattern);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn nested_map_in_array() {
        let expr = parse_expr("match x { [{name} as person, ...rest] => name }");
        let (_, arms) = assert_match(&expr);
        let (elements, rest) = assert_array_pattern(&arms[0].pattern);
        assert_eq!(elements.len(), 1);
        let (_, bind_whole) = assert_map_pattern(&elements[0]);
        assert_eq!(
            *bind_whole.expect("as binding"),
            Binding::Named("person".into())
        );
        assert_eq!(
            *rest.expect("rest binding"),
            Binding::Named("rest".into())
        );
    }
}

// ============================================================
// Newlines
// ============================================================

mod newlines {
    use super::*;

    #[test]
    fn arms_on_separate_lines() {
        let expr = parse_expr("match x {\n    1 => 'one',\n    2 => 'two',\n    _ => 'other'\n}");
        let (_, arms) = assert_match(&expr);
        assert_eq!(arms.len(), 3);
    }

    #[test]
    fn newline_after_fat_arrow() {
        let expr = parse_expr("match x {\n    _ =>\n        42\n}");
        let (_, arms) = assert_match(&expr);
        assert!(is_int(&arms[0].result, 42));
    }

    #[test]
    fn multiline_array_pattern() {
        let expr = parse_expr("match x {\n    [\n        a,\n        b,\n        ...rest\n    ] => a\n}");
        let (_, arms) = assert_match(&expr);
        let (elements, rest) = assert_array_pattern(&arms[0].pattern);
        assert_eq!(elements.len(), 2);
        assert!(rest.is_some());
    }

    #[test]
    fn multiline_map_pattern() {
        let expr = parse_expr(
            "match x {\n    {\n        name: n,\n        age: a\n    } => n\n}",
        );
        let (_, arms) = assert_match(&expr);
        let (entries, _) = assert_map_pattern(&arms[0].pattern);
        assert_eq!(entries.len(), 2);
    }
}

// ============================================================
// Errors
// ============================================================

mod errors {
    use super::*;

    #[test]
    fn missing_open_brace() {
        let err = parse_err("match x 1 => 2");
        assert!(
            err.contains("Expected {") || err.contains("unexpected"),
            "error was: {err}"
        );
    }

    #[test]
    fn missing_fat_arrow() {
        let err = parse_err("match x { 1 2 }");
        assert!(
            err.contains("Expected =>") || err.contains("unexpected"),
            "error was: {err}"
        );
    }

    #[test]
    fn missing_close_brace() {
        let err = parse_err("match x { 1 => 2");
        assert!(
            err.contains("end of input") || err.contains("Expected }"),
            "error was: {err}"
        );
    }

    #[test]
    fn invalid_type_constraint() {
        let err = parse_err("match x { n is Bogus => n }");
        assert!(
            err.contains("unexpected") || err.contains("type constraint"),
            "error was: {err}"
        );
    }

    #[test]
    fn negative_non_literal() {
        let err = parse_err("match x { -'hello' => 1 }");
        assert!(
            err.contains("unexpected") || err.contains("negative literal"),
            "error was: {err}"
        );
    }
}
