use frost_parse::lex::Token;
use logos::Logos;

fn lex<'a>(input: &'a str) -> Vec<Token<'a>> {
    Token::lexer(input)
        .map(|r| r.expect(&format!("unexpected token in: {input:?}")))
        .collect()
}

fn lex_one<'a>(input: &'a str) -> Token<'a> {
    let tokens = lex(input);
    assert_eq!(
        tokens.len(),
        1,
        "expected 1 token from {input:?}, got {tokens:?}"
    );
    tokens.into_iter().next().unwrap()
}

// ---- Keywords ----

#[test]
fn keywords() {
    assert_eq!(lex_one("as"), Token::KwAs);
    assert_eq!(lex_one("def"), Token::KwDef);
    assert_eq!(lex_one("defn"), Token::KwDefn);
    assert_eq!(lex_one("do"), Token::KwDo);
    assert_eq!(lex_one("elif"), Token::KwElif);
    assert_eq!(lex_one("else"), Token::KwElse);
    assert_eq!(lex_one("export"), Token::KwExport);
    assert_eq!(lex_one("false"), Token::KwFalse);
    assert_eq!(lex_one("filter"), Token::KwFilter);
    assert_eq!(lex_one("fn"), Token::KwFn);
    assert_eq!(lex_one("foreach"), Token::KwForeach);
    assert_eq!(lex_one("if"), Token::KwIf);
    assert_eq!(lex_one("init"), Token::KwInit);
    assert_eq!(lex_one("is"), Token::KwIs);
    assert_eq!(lex_one("map"), Token::KwMap);
    assert_eq!(lex_one("match"), Token::KwMatch);
    assert_eq!(lex_one("null"), Token::KwNull);
    assert_eq!(lex_one("reduce"), Token::KwReduce);
    assert_eq!(lex_one("true"), Token::KwTrue);
    assert_eq!(lex_one("with"), Token::KwWith);
}

#[test]
fn keyword_prefix_is_identifier() {
    // "defn" is a keyword, but "define" is an identifier
    assert_eq!(lex_one("define"), Token::Identifier("define"));
    assert_eq!(lex_one("iffy"), Token::Identifier("iffy"));
    assert_eq!(lex_one("trueish"), Token::Identifier("trueish"));
    assert_eq!(lex_one("nullify"), Token::Identifier("nullify"));
    assert_eq!(lex_one("format"), Token::Identifier("format"));
    assert_eq!(lex_one("matching"), Token::Identifier("matching"));
}

// ---- Keyword operators ----

#[test]
fn keyword_operators() {
    assert_eq!(lex_one("and"), Token::OpAnd);
    assert_eq!(lex_one("or"), Token::OpOr);
    assert_eq!(lex_one("not"), Token::OpNot);
}

// ---- Punctuation ----

#[test]
fn punctuation() {
    assert_eq!(lex_one(":"), Token::Colon);
    assert_eq!(lex_one(";"), Token::Semicolon);
    assert_eq!(lex_one(","), Token::Comma);
    assert_eq!(lex_one("->"), Token::SlimArrow);
    assert_eq!(lex_one("=>"), Token::FatArrow);
    assert_eq!(lex_one("("), Token::OpenParen);
    assert_eq!(lex_one(")"), Token::CloseParen);
    assert_eq!(lex_one("["), Token::OpenBracket);
    assert_eq!(lex_one("]"), Token::CloseBracket);
    assert_eq!(lex_one("{"), Token::OpenBrace);
    assert_eq!(lex_one("}"), Token::CloseBrace);
    assert_eq!(lex_one("..."), Token::DotDotDot);
    assert_eq!(lex_one("="), Token::Assign);
    assert_eq!(lex_one("|"), Token::Pipe);
}

#[test]
fn dollar_paren() {
    assert_eq!(lex_one("$("), Token::DollarParen);
}

#[test]
fn newline_is_token() {
    assert_eq!(lex_one("\n"), Token::Newline);
}

// ---- Operators ----

#[test]
fn arithmetic_operators() {
    assert_eq!(lex_one("+"), Token::OpPlus);
    assert_eq!(lex_one("-"), Token::OpMinus);
    assert_eq!(lex_one("*"), Token::OpTimes);
    assert_eq!(lex_one("/"), Token::OpDiv);
    assert_eq!(lex_one("%"), Token::OpMod);
}

#[test]
fn comparison_operators() {
    assert_eq!(lex_one("=="), Token::OpEq);
    assert_eq!(lex_one("!="), Token::OpNeq);
    assert_eq!(lex_one("<"), Token::OpLt);
    assert_eq!(lex_one("<="), Token::OpLte);
    assert_eq!(lex_one(">"), Token::OpGt);
    assert_eq!(lex_one(">="), Token::OpGte);
}

#[test]
fn dot_and_thread() {
    assert_eq!(lex_one("."), Token::OpDot);
    assert_eq!(lex_one("@"), Token::OpThread);
}

// ---- Integer literals ----

#[test]
fn integer_literals() {
    assert_eq!(lex_one("0"), Token::IntLiteral(0));
    assert_eq!(lex_one("42"), Token::IntLiteral(42));
    assert_eq!(lex_one("123456789"), Token::IntLiteral(123456789));
}

// ---- Float literals ----

#[test]
fn float_fixed() {
    assert_eq!(lex_one("3.14"), Token::FloatLiteral(3.14));
    assert_eq!(lex_one("0.5"), Token::FloatLiteral(0.5));
    assert_eq!(lex_one("100.0"), Token::FloatLiteral(100.0));
}

#[test]
fn float_scientific() {
    assert_eq!(lex_one("1e10"), Token::FloatLiteral(1e10));
    assert_eq!(lex_one("1e+10"), Token::FloatLiteral(1e10));
    assert_eq!(lex_one("1e-10"), Token::FloatLiteral(1e-10));
    assert_eq!(lex_one("5E3"), Token::FloatLiteral(5e3));
}

#[test]
fn float_fixed_scientific() {
    assert_eq!(lex_one("1.5e10"), Token::FloatLiteral(1.5e10));
    assert_eq!(lex_one("1.5e+10"), Token::FloatLiteral(1.5e10));
    assert_eq!(lex_one("1.5e-10"), Token::FloatLiteral(1.5e-10));
}

#[test]
fn float_leading_dot() {
    assert_eq!(lex_one(".5"), Token::FloatLiteral(0.5));
    assert_eq!(lex_one(".123"), Token::FloatLiteral(0.123));
    assert_eq!(lex_one(".5e2"), Token::FloatLiteral(0.5e2));
}

// ---- Identifiers ----

#[test]
fn identifiers() {
    assert_eq!(lex_one("x"), Token::Identifier("x"));
    assert_eq!(lex_one("foo"), Token::Identifier("foo"));
    assert_eq!(lex_one("hello_world"), Token::Identifier("hello_world"));
    assert_eq!(lex_one("_private"), Token::Identifier("_private"));
    assert_eq!(lex_one("_"), Token::Identifier("_"));
    assert_eq!(lex_one("camelCase"), Token::Identifier("camelCase"));
    assert_eq!(lex_one("item42"), Token::Identifier("item42"));
    assert_eq!(lex_one("ALL_CAPS"), Token::Identifier("ALL_CAPS"));
}

// ---- Whitespace skipping ----

#[test]
fn spaces_are_skipped() {
    assert_eq!(lex("  42  "), vec![Token::IntLiteral(42)]);
}

#[test]
fn tabs_are_skipped() {
    assert_eq!(lex("\t42\t"), vec![Token::IntLiteral(42)]);
}

#[test]
fn mixed_whitespace_skipped() {
    assert_eq!(
        lex("  def \t x  =  42"),
        vec![
            Token::KwDef,
            Token::Identifier("x"),
            Token::Assign,
            Token::IntLiteral(42)
        ]
    );
}

// ---- Comment skipping ----

#[test]
fn comment_skipped() {
    assert_eq!(lex("42 # this is a comment"), vec![Token::IntLiteral(42)]);
}

#[test]
fn comment_preserves_newline() {
    assert_eq!(
        lex("42 # comment\n43"),
        vec![Token::IntLiteral(42), Token::Newline, Token::IntLiteral(43)]
    );
}

#[test]
fn comment_only_line() {
    assert_eq!(lex("# just a comment"), vec![]);
}

// ---- Multi-token sequences ----

#[test]
fn simple_def() {
    assert_eq!(
        lex("def x = 42"),
        vec![
            Token::KwDef,
            Token::Identifier("x"),
            Token::Assign,
            Token::IntLiteral(42)
        ]
    );
}

#[test]
fn function_def() {
    assert_eq!(
        lex("defn add(x, y) -> x + y"),
        vec![
            Token::KwDefn,
            Token::Identifier("add"),
            Token::OpenParen,
            Token::Identifier("x"),
            Token::Comma,
            Token::Identifier("y"),
            Token::CloseParen,
            Token::SlimArrow,
            Token::Identifier("x"),
            Token::OpPlus,
            Token::Identifier("y"),
        ]
    );
}

#[test]
fn if_expression() {
    assert_eq!(
        lex("if x: 1 else: 2"),
        vec![
            Token::KwIf,
            Token::Identifier("x"),
            Token::Colon,
            Token::IntLiteral(1),
            Token::KwElse,
            Token::Colon,
            Token::IntLiteral(2),
        ]
    );
}

#[test]
fn array_literal() {
    assert_eq!(
        lex("[1, 2, 3]"),
        vec![
            Token::OpenBracket,
            Token::IntLiteral(1),
            Token::Comma,
            Token::IntLiteral(2),
            Token::Comma,
            Token::IntLiteral(3),
            Token::CloseBracket,
        ]
    );
}

#[test]
fn map_literal() {
    assert_eq!(
        lex("{ foo: 42 }"),
        vec![
            Token::OpenBrace,
            Token::Identifier("foo"),
            Token::Colon,
            Token::IntLiteral(42),
            Token::CloseBrace,
        ]
    );
}

#[test]
fn threading_pipeline() {
    assert_eq!(
        lex("x @ f() @ g()"),
        vec![
            Token::Identifier("x"),
            Token::OpThread,
            Token::Identifier("f"),
            Token::OpenParen,
            Token::CloseParen,
            Token::OpThread,
            Token::Identifier("g"),
            Token::OpenParen,
            Token::CloseParen,
        ]
    );
}

#[test]
fn match_expression() {
    assert_eq!(
        lex("match x { 1 | 2 => true, _ => false }"),
        vec![
            Token::KwMatch,
            Token::Identifier("x"),
            Token::OpenBrace,
            Token::IntLiteral(1),
            Token::Pipe,
            Token::IntLiteral(2),
            Token::FatArrow,
            Token::KwTrue,
            Token::Comma,
            Token::Identifier("_"),
            Token::FatArrow,
            Token::KwFalse,
            Token::CloseBrace,
        ]
    );
}

#[test]
fn multiline_with_newlines() {
    assert_eq!(
        lex("def x = 1\ndef y = 2"),
        vec![
            Token::KwDef,
            Token::Identifier("x"),
            Token::Assign,
            Token::IntLiteral(1),
            Token::Newline,
            Token::KwDef,
            Token::Identifier("y"),
            Token::Assign,
            Token::IntLiteral(2),
        ]
    );
}

#[test]
fn semicolons_as_separators() {
    assert_eq!(
        lex("def x = 1; def y = 2"),
        vec![
            Token::KwDef,
            Token::Identifier("x"),
            Token::Assign,
            Token::IntLiteral(1),
            Token::Semicolon,
            Token::KwDef,
            Token::Identifier("y"),
            Token::Assign,
            Token::IntLiteral(2),
        ]
    );
}

#[test]
fn lambda_with_rest() {
    assert_eq!(
        lex("fn x, ...rest -> x"),
        vec![
            Token::KwFn,
            Token::Identifier("x"),
            Token::Comma,
            Token::DotDotDot,
            Token::Identifier("rest"),
            Token::SlimArrow,
            Token::Identifier("x"),
        ]
    );
}

#[test]
fn abbreviated_lambda() {
    assert_eq!(
        lex("$(x + 1)"),
        vec![
            Token::DollarParen,
            Token::Identifier("x"),
            Token::OpPlus,
            Token::IntLiteral(1),
            Token::CloseParen,
        ]
    );
}

#[test]
fn dot_access() {
    assert_eq!(
        lex("foo.bar.baz"),
        vec![
            Token::Identifier("foo"),
            Token::OpDot,
            Token::Identifier("bar"),
            Token::OpDot,
            Token::Identifier("baz"),
        ]
    );
}

#[test]
fn negative_number_is_two_tokens() {
    // Unary minus is handled by the parser, not the lexer
    assert_eq!(lex("-42"), vec![Token::OpMinus, Token::IntLiteral(42)]);
}

#[test]
fn comparison_chain() {
    assert_eq!(
        lex("a == b != c <= d >= e < f > g"),
        vec![
            Token::Identifier("a"),
            Token::OpEq,
            Token::Identifier("b"),
            Token::OpNeq,
            Token::Identifier("c"),
            Token::OpLte,
            Token::Identifier("d"),
            Token::OpGte,
            Token::Identifier("e"),
            Token::OpLt,
            Token::Identifier("f"),
            Token::OpGt,
            Token::Identifier("g"),
        ]
    );
}

#[test]
fn logical_operators_in_context() {
    assert_eq!(
        lex("a and b or not c"),
        vec![
            Token::Identifier("a"),
            Token::OpAnd,
            Token::Identifier("b"),
            Token::OpOr,
            Token::OpNot,
            Token::Identifier("c"),
        ]
    );
}

// ---- Ambiguity: longest match ----

#[test]
fn arrow_vs_minus_gt() {
    // -> should be SlimArrow, not OpMinus + OpGt
    assert_eq!(lex("->"), vec![Token::SlimArrow]);
}

#[test]
fn fat_arrow_vs_eq_gt() {
    // => should be FatArrow, not Assign + OpGt
    assert_eq!(lex("=>"), vec![Token::FatArrow]);
}

#[test]
fn eq_vs_assign() {
    // == is OpEq, = is Assign
    assert_eq!(lex("=="), vec![Token::OpEq]);
    assert_eq!(lex("="), vec![Token::Assign]);
}

#[test]
fn dots_disambiguation() {
    // . is OpDot, ... is DotDotDot
    assert_eq!(lex("."), vec![Token::OpDot]);
    assert_eq!(lex("..."), vec![Token::DotDotDot]);
}

#[test]
fn float_vs_dot_int() {
    // 42.0 should be a float, not int + dot + int
    assert_eq!(lex("42.0"), vec![Token::FloatLiteral(42.0)]);
}

#[test]
fn leading_dot_float_vs_dot() {
    // .5 should be a float, not dot + int
    assert_eq!(lex(".5"), vec![Token::FloatLiteral(0.5)]);
}

// ---- Edge cases ----

#[test]
fn empty_input() {
    assert_eq!(lex(""), vec![]);
}

#[test]
fn only_whitespace() {
    assert_eq!(lex("   \t\t   "), vec![]);
}

#[test]
fn consecutive_newlines() {
    assert_eq!(
        lex("\n\n\n"),
        vec![Token::Newline, Token::Newline, Token::Newline]
    );
}

#[test]
fn dollar_paren_vs_paren() {
    // $( is DollarParen, ( alone is OpenParen
    assert_eq!(
        lex("$(x)(y)"),
        vec![
            Token::DollarParen,
            Token::Identifier("x"),
            Token::CloseParen,
            Token::OpenParen,
            Token::Identifier("y"),
            Token::CloseParen,
        ]
    );
}

// ---- String literals ----

#[test]
fn raw_string_single() {
    assert_eq!(lex_one("R'(hello)'"), Token::RawStringLiteral("hello"));
}

#[test]
fn raw_string_double() {
    assert_eq!(lex_one(r#"R"(hello)""#), Token::RawStringLiteral("hello"));
}

#[test]
fn raw_string_preserves_backslashes() {
    assert_eq!(lex_one(r"R'(hello\nworld)'"), Token::RawStringLiteral(r"hello\nworld"));
}

#[test]
fn simple_string_single() {
    assert_eq!(lex_one("'hello'"), Token::SimpleStringLiteral("hello"));
}

#[test]
fn simple_string_double() {
    assert_eq!(lex_one(r#""hello""#), Token::SimpleStringLiteral("hello"));
}

#[test]
fn simple_string_empty() {
    assert_eq!(lex_one("''"), Token::SimpleStringLiteral(""));
    assert_eq!(lex_one(r#""""#), Token::SimpleStringLiteral(""));
}

#[test]
fn simple_string_with_escapes() {
    // Escapes are preserved raw — parser handles them
    assert_eq!(lex_one(r"'hello\nworld'"), Token::SimpleStringLiteral(r"hello\nworld"));
    assert_eq!(lex_one(r"'tab\there'"), Token::SimpleStringLiteral(r"tab\there"));
}

#[test]
fn simple_string_escaped_quote() {
    assert_eq!(lex_one(r"'it\'s'"), Token::SimpleStringLiteral(r"it\'s"));
    assert_eq!(lex_one(r#""say \"hi\"""#), Token::SimpleStringLiteral(r#"say \"hi\""#));
}

#[test]
fn simple_string_escaped_backslash() {
    assert_eq!(lex_one(r"'back\\slash'"), Token::SimpleStringLiteral(r"back\\slash"));
}

#[test]
fn multiline_string_double() {
    let input = r#""""
    hello
    world
    """"#;
    let tok = lex_one(input);
    assert!(matches!(tok, Token::MultilineStringLiteral(_)));
    if let Token::MultilineStringLiteral(content) = tok {
        assert!(content.contains("hello"));
        assert!(content.contains("world"));
        assert!(content.contains('\n'));
    }
}

#[test]
fn multiline_string_single() {
    let input = "'''\n    hello\n    world\n    '''"; // can't use raw string here — ''' conflicts
    let tok = lex_one(input);
    assert!(matches!(tok, Token::MultilineStringLiteral(_)));
}

// ---- Format string literals ----

#[test]
fn format_string_simple() {
    assert_eq!(
        lex_one("$'hello'"),
        Token::FormatStringLiteral("hello")
    );
}

#[test]
fn format_string_double_quote() {
    assert_eq!(
        lex_one("$\"hello\""),
        Token::FormatStringLiteral("hello")
    );
}

#[test]
fn format_string_with_interpolation() {
    assert_eq!(
        lex_one("$'hello, ${name}'"),
        Token::FormatStringLiteral("hello, ${name}")
    );
}

#[test]
fn format_string_with_expression_interpolation() {
    assert_eq!(
        lex_one("$'result: ${1 + 2}'"),
        Token::FormatStringLiteral("result: ${1 + 2}")
    );
}

#[test]
fn format_string_nested_braces_in_interpolation() {
    // Map literal inside interpolation: { foo: 1 } has braces
    assert_eq!(
        lex_one("$'value: ${{ foo: 1 }}'"),
        Token::FormatStringLiteral("value: ${{ foo: 1 }}")
    );
}

#[test]
fn format_string_string_inside_interpolation_same_quote() {
    // String with same quote style inside interpolation
    assert_eq!(
        lex_one("$'result: ${map['key']}'"),
        Token::FormatStringLiteral("result: ${map['key']}")
    );
}

#[test]
fn format_string_string_inside_interpolation_other_quote() {
    assert_eq!(
        lex_one(r#"$"result: ${map["key"]}""#),
        Token::FormatStringLiteral(r#"result: ${map["key"]}"#)
    );
}

#[test]
fn format_string_escaped_dollar() {
    assert_eq!(
        lex_one(r"$'price: \${5}'"),
        Token::FormatStringLiteral(r"price: \${5}")
    );
}

#[test]
fn format_string_bare_dollar() {
    // $ not followed by { is literal
    assert_eq!(
        lex_one("$'price: $5'"),
        Token::FormatStringLiteral("price: $5")
    );
}

#[test]
fn format_string_escaped_quote() {
    assert_eq!(
        lex_one(r"$'it\'s here'"),
        Token::FormatStringLiteral(r"it\'s here")
    );
}

#[test]
fn format_string_escaped_backslash_before_quote() {
    // \\' should be escaped backslash then closing quote
    assert_eq!(
        lex_one("$'back\\\\'"),
        Token::FormatStringLiteral("back\\\\")
    );
}

#[test]
fn format_string_multiple_interpolations() {
    assert_eq!(
        lex_one("$'${a} and ${b}'"),
        Token::FormatStringLiteral("${a} and ${b}")
    );
}

#[test]
fn format_string_nested_format_string_in_interpolation() {
    // Format string inside interpolation: ${$"inner"}
    assert_eq!(
        lex_one(r#"$'outer: ${$"inner"}'  "#.trim()),
        Token::FormatStringLiteral(r#"outer: ${$"inner"}"#)
    );
}

#[test]
fn format_string_complex_expression() {
    // if expression inside interpolation
    assert_eq!(
        lex_one(r#"$"${if true: "yes" else: "no"}""#),
        Token::FormatStringLiteral(r#"${if true: "yes" else: "no"}"#)
    );
}

#[test]
fn format_string_empty() {
    assert_eq!(lex_one("$''"), Token::FormatStringLiteral(""));
    assert_eq!(lex_one("$\"\""), Token::FormatStringLiteral(""));
}

#[test]
fn format_string_brace_in_string_in_interpolation() {
    // } inside a string inside interpolation shouldn't close the interpolation
    assert_eq!(
        lex_one("$'${f(\"}\")}'"),
        Token::FormatStringLiteral("${f(\"}\")}")
    );
}

#[test]
fn format_string_nested_interpolation_with_braces() {
    // Deeply nested braces: ${fn -> { def x = { a: 1 }; x }}
    assert_eq!(
        lex_one("$'${fn -> { def x = { a: 1 }; x }}'"),
        Token::FormatStringLiteral("${fn -> { def x = { a: 1 }; x }}")
    );
}

#[test]
fn format_string_is_single_token() {
    // The whole format string is one token, other tokens can follow
    assert_eq!(
        lex("$'hello' + $'world'"),
        vec![
            Token::FormatStringLiteral("hello"),
            Token::OpPlus,
            Token::FormatStringLiteral("world"),
        ]
    );
}

// ---- Dollar identifiers ----

#[test]
fn dollar_bare() {
    assert_eq!(lex_one("$"), Token::DollarIdentifier("$"));
}

#[test]
fn dollar_digits() {
    assert_eq!(lex_one("$1"), Token::DollarIdentifier("$1"));
    assert_eq!(lex_one("$2"), Token::DollarIdentifier("$2"));
    assert_eq!(lex_one("$9"), Token::DollarIdentifier("$9"));
}

#[test]
fn dollar_dollar() {
    assert_eq!(lex_one("$$"), Token::DollarIdentifier("$$"));
}

#[test]
fn dollar_in_abbreviated_lambda() {
    assert_eq!(
        lex("$($ * 2)"),
        vec![
            Token::DollarParen,
            Token::DollarIdentifier("$"),
            Token::OpTimes,
            Token::IntLiteral(2),
            Token::CloseParen,
        ]
    );
}

#[test]
fn dollar_numbered_in_abbreviated_lambda() {
    assert_eq!(
        lex("$($1 + $2)"),
        vec![
            Token::DollarParen,
            Token::DollarIdentifier("$1"),
            Token::OpPlus,
            Token::DollarIdentifier("$2"),
            Token::CloseParen,
        ]
    );
}

#[test]
fn dollar_rest_in_abbreviated_lambda() {
    assert_eq!(
        lex("$($$ )"),
        vec![
            Token::DollarParen,
            Token::DollarIdentifier("$$"),
            Token::CloseParen,
        ]
    );
}

// ---- Full program lexing ----

#[test]
fn lex_prelude_tap() {
    // From the Frost prelude: defn tap(a, f) -> { f(a) ; a }
    assert_eq!(
        lex("defn tap(a, f) -> { f(a) ; a }"),
        vec![
            Token::KwDefn,
            Token::Identifier("tap"),
            Token::OpenParen,
            Token::Identifier("a"),
            Token::Comma,
            Token::Identifier("f"),
            Token::CloseParen,
            Token::SlimArrow,
            Token::OpenBrace,
            Token::Identifier("f"),
            Token::OpenParen,
            Token::Identifier("a"),
            Token::CloseParen,
            Token::Semicolon,
            Token::Identifier("a"),
            Token::CloseBrace,
        ]
    );
}

#[test]
fn lex_prelude_curry() {
    // defn curry(f, ...outer) -> fn ...inner -> f @ call(outer + inner)
    assert_eq!(
        lex("defn curry(f, ...outer) -> fn ...inner -> f @ call(outer + inner)"),
        vec![
            Token::KwDefn,
            Token::Identifier("curry"),
            Token::OpenParen,
            Token::Identifier("f"),
            Token::Comma,
            Token::DotDotDot,
            Token::Identifier("outer"),
            Token::CloseParen,
            Token::SlimArrow,
            Token::KwFn,
            Token::DotDotDot,
            Token::Identifier("inner"),
            Token::SlimArrow,
            Token::Identifier("f"),
            Token::OpThread,
            Token::Identifier("call"),
            Token::OpenParen,
            Token::Identifier("outer"),
            Token::OpPlus,
            Token::Identifier("inner"),
            Token::CloseParen,
        ]
    );
}

#[test]
fn lex_prelude_compose_fragment() {
    // A chunk from compose that exercises format strings, if/else, map iteration
    let src = r#"defn require_fn(f) -> {
    assert(is_function(f), $'Compose requires functions, got ${type(f)}')
}

foreach [f, g] + rest with require_fn

if len(rest) == 0: compose2(f, g)
else: call(compose, [ compose2(f, g) ] + rest )"#;

    let tokens = lex(src);

    // Check key tokens are present without asserting exact sequence
    assert!(tokens.contains(&Token::KwDefn));
    assert!(tokens.contains(&Token::KwForeach));
    assert!(tokens.contains(&Token::KwIf));
    assert!(tokens.contains(&Token::KwElse));
    assert!(tokens.contains(&Token::OpEq));
    assert!(tokens.contains(&Token::Colon));
    assert!(tokens.contains(&Token::KwWith));

    // The format string should be captured as a single token
    assert!(tokens.iter().any(|t| matches!(t, Token::FormatStringLiteral(_))));

    // Verify the format string content includes the interpolation
    if let Some(Token::FormatStringLiteral(s)) = tokens.iter().find(|t| matches!(t, Token::FormatStringLiteral(_))) {
        assert!(s.contains("${type(f)}"));
        assert!(s.contains("Compose requires functions"));
    }
}

#[test]
fn lex_multiline_program() {
    let src = r#"def x = 42
def y = x + 1
if y > 42: true else: false"#;
    let tokens = lex(src);

    let newline_count = tokens.iter().filter(|t| **t == Token::Newline).count();
    assert_eq!(newline_count, 2);

    assert_eq!(tokens[0], Token::KwDef);
    assert_eq!(tokens[1], Token::Identifier("x"));
    assert_eq!(tokens[2], Token::Assign);
    assert_eq!(tokens[3], Token::IntLiteral(42));
    assert_eq!(tokens[4], Token::Newline);

    assert_eq!(tokens[5], Token::KwDef);
    assert_eq!(tokens[6], Token::Identifier("y"));
    assert_eq!(tokens[7], Token::Assign);
    assert_eq!(tokens[8], Token::Identifier("x"));
    assert_eq!(tokens[9], Token::OpPlus);
    assert_eq!(tokens[10], Token::IntLiteral(1));
    assert_eq!(tokens[11], Token::Newline);

    assert_eq!(tokens[12], Token::KwIf);
    assert_eq!(tokens[13], Token::Identifier("y"));
    assert_eq!(tokens[14], Token::OpGt);
    assert_eq!(tokens[15], Token::IntLiteral(42));
    assert_eq!(tokens[16], Token::Colon);
    assert_eq!(tokens[17], Token::KwTrue);
    assert_eq!(tokens[18], Token::KwElse);
    assert_eq!(tokens[19], Token::Colon);
    assert_eq!(tokens[20], Token::KwFalse);
}

#[test]
fn lex_match_with_patterns() {
    let src = r#"match v {
    n is Int if: n > 0 => n,
    [first, ...rest] => first,
    _ => null
}"#;
    let tokens = lex(src);

    assert!(tokens.contains(&Token::KwMatch));
    assert!(tokens.contains(&Token::KwIs));
    assert!(tokens.contains(&Token::KwIf));
    assert!(tokens.contains(&Token::FatArrow));
    assert!(tokens.contains(&Token::DotDotDot));
    assert!(tokens.contains(&Token::KwNull));
    assert!(tokens.contains(&Token::Comma));
}

#[test]
fn lex_map_filter_reduce() {
    let src = "def result = reduce (filter (map [1, 2, 3] with fn x -> x * 2) with fn x -> x > 2) with plus";
    let tokens = lex(src);

    assert!(tokens.contains(&Token::KwMap));
    assert!(tokens.contains(&Token::KwFilter));
    assert!(tokens.contains(&Token::KwReduce));
    assert!(tokens.contains(&Token::KwWith));
    assert!(tokens.contains(&Token::KwFn));
    assert!(tokens.contains(&Token::SlimArrow));
    assert_eq!(tokens.iter().filter(|t| **t == Token::KwWith).count(), 3);
}

#[test]
fn lex_export_def_with_destructure() {
    let src = "export def [a, b, ...rest] = some_array";
    assert_eq!(
        lex(src),
        vec![
            Token::KwExport,
            Token::KwDef,
            Token::OpenBracket,
            Token::Identifier("a"),
            Token::Comma,
            Token::Identifier("b"),
            Token::Comma,
            Token::DotDotDot,
            Token::Identifier("rest"),
            Token::CloseBracket,
            Token::Assign,
            Token::Identifier("some_array"),
        ]
    );
}

#[test]
fn lex_map_destructure() {
    let src = "def { name: n, age } = person";
    assert_eq!(
        lex(src),
        vec![
            Token::KwDef,
            Token::OpenBrace,
            Token::Identifier("name"),
            Token::Colon,
            Token::Identifier("n"),
            Token::Comma,
            Token::Identifier("age"),
            Token::CloseBrace,
            Token::Assign,
            Token::Identifier("person"),
        ]
    );
}

#[test]
fn lex_threading_pipeline_full() {
    let src = "data @ transform(fn x -> x * 2) @ select(fn x -> x > 5) @ fold(plus)";
    let tokens = lex(src);

    let thread_count = tokens.iter().filter(|t| **t == Token::OpThread).count();
    assert_eq!(thread_count, 3);
    assert!(tokens.contains(&Token::KwFn));
    assert!(tokens.contains(&Token::SlimArrow));
}
