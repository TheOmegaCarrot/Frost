mod helpers;

use frost_parse::ast::*;
use helpers::*;

fn assert_map_iter(expr: &Expr) -> (&Expr, &Expr) {
    match &expr.kind {
        ExprKind::MapIter {
            structure,
            operation,
        } => (structure, operation),
        other => panic!("expected MapIter, got {other:?}"),
    }
}

fn assert_filter(expr: &Expr) -> (&Expr, &Expr) {
    match &expr.kind {
        ExprKind::Filter {
            structure,
            operation,
        } => (structure, operation),
        other => panic!("expected Filter, got {other:?}"),
    }
}

fn assert_foreach(expr: &Expr) -> (&Expr, &Expr) {
    match &expr.kind {
        ExprKind::Foreach {
            structure,
            operation,
        } => (structure, operation),
        other => panic!("expected Foreach, got {other:?}"),
    }
}

fn assert_reduce(expr: &Expr) -> (&Expr, &Expr, Option<&Expr>) {
    match &expr.kind {
        ExprKind::Reduce {
            structure,
            operation,
            init,
        } => (structure, operation, init.as_deref()),
        other => panic!("expected Reduce, got {other:?}"),
    }
}

fn is_lambda(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::Lambda { .. })
}

fn is_array(expr: &Expr) -> bool {
    matches!(&expr.kind, ExprKind::Array(_))
}

// ============================================================
// map
// ============================================================

mod map_iter {
    use super::*;

    #[test]
    fn with_lambda() {
        let expr = parse_expr("map [1, 2, 3] with fn x -> x * 2");
        let (structure, operation) = assert_map_iter(&expr);
        assert!(is_array(structure));
        assert!(is_lambda(operation));
    }

    #[test]
    fn with_name() {
        let expr = parse_expr("map [1, 2] with double");
        let (structure, operation) = assert_map_iter(&expr);
        assert!(is_array(structure));
        assert!(matches!(&operation.kind, ExprKind::NameLookup(n) if n == "double"));
    }

    #[test]
    fn structure_is_expression() {
        let expr = parse_expr("map a + b with fn x -> x");
        let (structure, _) = assert_map_iter(&expr);
        assert!(is_binop(structure).is_some());
    }

    #[test]
    fn structure_is_variable() {
        let expr = parse_expr("map arr with fn x -> x");
        let (structure, _) = assert_map_iter(&expr);
        assert!(matches!(&structure.kind, ExprKind::NameLookup(n) if n == "arr"));
    }

    #[test]
    fn nested() {
        let expr = parse_expr("map map [1, 2] with fn x -> x with fn y -> y");
        let (structure, _) = assert_map_iter(&expr);
        assert_map_iter(structure);
    }

    #[test]
    fn in_def() {
        let program = parse("def result = map [1, 2] with fn x -> x * 2");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0].kind {
            StatementKind::Def { expr, .. } => {
                assert_map_iter(expr);
            }
            other => panic!("expected Def, got {other:?}"),
        }
    }
}

// ============================================================
// filter
// ============================================================

mod filter {
    use super::*;

    #[test]
    fn with_lambda() {
        let expr = parse_expr("filter [1, 2, 3] with fn x -> x > 1");
        let (structure, operation) = assert_filter(&expr);
        assert!(is_array(structure));
        assert!(is_lambda(operation));
    }

    #[test]
    fn with_name() {
        let expr = parse_expr("filter arr with is_positive");
        let (_, operation) = assert_filter(&expr);
        assert!(matches!(&operation.kind, ExprKind::NameLookup(n) if n == "is_positive"));
    }

    #[test]
    fn structure_is_expression() {
        let expr = parse_expr("filter a + b with fn x -> x > 0");
        let (structure, _) = assert_filter(&expr);
        assert!(is_binop(structure).is_some());
    }
}

// ============================================================
// foreach
// ============================================================

mod foreach {
    use super::*;

    #[test]
    fn with_lambda() {
        let expr = parse_expr("foreach [1, 2, 3] with fn x -> print(x)");
        let (structure, operation) = assert_foreach(&expr);
        assert!(is_array(structure));
        assert!(is_lambda(operation));
    }

    #[test]
    fn with_name() {
        let expr = parse_expr("foreach items with print");
        let (_, operation) = assert_foreach(&expr);
        assert!(matches!(&operation.kind, ExprKind::NameLookup(n) if n == "print"));
    }
}

// ============================================================
// reduce
// ============================================================

mod reduce {
    use super::*;

    #[test]
    fn without_init() {
        let expr = parse_expr("reduce [1, 2, 3] with fn (acc, x) -> acc + x");
        let (structure, operation, init) = assert_reduce(&expr);
        assert!(is_array(structure));
        assert!(is_lambda(operation));
        assert!(init.is_none());
    }

    #[test]
    fn with_init() {
        let expr = parse_expr("reduce [1, 2, 3] init: 0 with fn (acc, x) -> acc + x");
        let (structure, operation, init) = assert_reduce(&expr);
        assert!(is_array(structure));
        assert!(is_lambda(operation));
        assert!(is_int(init.expect("init"), 0));
    }

    #[test]
    fn init_is_expression() {
        let expr = parse_expr("reduce arr init: 1 + 2 with fn (acc, x) -> acc + x");
        let (_, _, init) = assert_reduce(&expr);
        assert!(is_binop(init.expect("init")).is_some());
    }

    #[test]
    fn with_name_operation() {
        let expr = parse_expr("reduce [1, 2, 3] with add");
        let (_, operation, _) = assert_reduce(&expr);
        assert!(matches!(&operation.kind, ExprKind::NameLookup(n) if n == "add"));
    }

    #[test]
    fn structure_is_expression() {
        let expr = parse_expr("reduce a + b with fn (acc, x) -> acc + x");
        let (structure, _, _) = assert_reduce(&expr);
        assert!(is_binop(structure).is_some());
    }
}

// ============================================================
// Newlines
// ============================================================

mod newlines {
    use super::*;

    #[test]
    fn newline_before_with() {
        let expr = parse_expr("map [1, 2]\nwith fn x -> x");
        let (structure, operation) = assert_map_iter(&expr);
        assert!(is_array(structure));
        assert!(is_lambda(operation));
    }

    #[test]
    fn newline_after_with() {
        let expr = parse_expr("map [1, 2] with\nfn x -> x");
        let (_, operation) = assert_map_iter(&expr);
        assert!(is_lambda(operation));
    }

    #[test]
    fn reduce_newline_before_init() {
        let expr = parse_expr("reduce arr\ninit: 0\nwith fn (acc, x) -> acc + x");
        let (_, _, init) = assert_reduce(&expr);
        assert!(is_int(init.expect("init"), 0));
    }

    #[test]
    fn reduce_newline_after_init_colon() {
        let expr = parse_expr("reduce arr init:\n0 with fn (acc, x) -> acc + x");
        let (_, _, init) = assert_reduce(&expr);
        assert!(is_int(init.expect("init"), 0));
    }
}

// ============================================================
// Errors
// ============================================================

mod errors {
    use super::*;

    #[test]
    fn missing_with() {
        let err = parse_err("map [1, 2] fn x -> x");
        assert!(
            err.contains("Expected with") || err.contains("unexpected"),
            "error was: {err}"
        );
    }

    #[test]
    fn missing_operation() {
        let err = parse_err("map [1, 2] with");
        assert!(
            err.contains("end of input") || err.contains("unexpected"),
            "error was: {err}"
        );
    }

    #[test]
    fn reduce_missing_with() {
        let err = parse_err("reduce [1, 2] init: 0 fn (a, x) -> a + x");
        assert!(
            err.contains("Expected with") || err.contains("unexpected"),
            "error was: {err}"
        );
    }

    // The seed clause comes before `with`, not after it.
    #[test]
    fn reduce_init_after_with() {
        let err = parse_err("reduce xs with f init: 0");
        assert!(
            err.contains("unexpected") || err.contains("Expected"),
            "error was: {err}"
        );
    }

    // `init:` is a reduce-only clause; map has no seed.
    #[test]
    fn map_rejects_init() {
        let err = parse_err("map xs init: 0 with f");
        assert!(
            err.contains("Expected with") || err.contains("unexpected"),
            "error was: {err}"
        );
    }

    // The seed clause is `init:`; the colon is required.
    #[test]
    fn reduce_init_requires_colon() {
        let err = parse_err("reduce xs init 0 with f");
        assert!(
            err.contains("Expected :") || err.contains("unexpected") || err.contains("Expected"),
            "error was: {err}"
        );
    }
}
