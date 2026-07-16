// -- Source location --

use std::ops::Range;

use serde::Serialize;

/// A range in source code, from start (inclusive) to end (exclusive).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SourceSpan {
    // byte offsets into the source
    pub start: usize,
    pub end: usize,
}

impl From<Range<usize>> for SourceSpan {
    fn from(value: Range<usize>) -> Self {
        Self {
            start: value.start,
            end: value.end,
        }
    }
}

// -- Spanned --


/// Pairs an AST payload with its source span.
/// Every node's span encloses the union of its children's spans.
#[derive(Clone, Debug, Serialize)]
pub struct Spanned<T> {
    pub node: T,
    pub span: SourceSpan,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: SourceSpan) -> Self {
        Self { node, span }
    }
}

// -- Binding --

/// A name binding: either a named identifier or a discard (`_`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Binding {
    Named(String),
    Discarded,
}

// -- Program --

/// A program is a sequence of statements.
#[derive(Clone, Debug, Serialize)]
pub struct Program {
    pub statements: Vec<Spanned<Statement>>,
}

// -- Statements --

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Statement {
    /// `def name = expr` or `export def name = expr`
    Def {
        exported: bool,
        destructure: Spanned<Destructure>,
        expr: Spanned<Expr>,
    },
    /// A bare expression executed for its side effects.
    Expr(Spanned<Expr>),
}

// -- Expressions --

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Expr {
    /// A literal value: `42`, `3.14`, `"hello"`, `true`, `null`.
    Literal(Literal),
    /// A variable reference: `foo`.
    NameLookup(String),
    /// A binary operation: `a + b`, `x == y`, `p and q`.
    BinOp {
        left: Box<Spanned<Expr>>,
        op: Spanned<BinOp>,
        right: Box<Spanned<Expr>>,
    },
    /// A unary operation: `-x`, `not x`.
    UnaryOp {
        op: Spanned<UnaryOp>,
        operand: Box<Spanned<Expr>>,
    },
    /// `if cond: then elif cond2: then2 else: fallback`
    If {
        condition: Box<Spanned<Expr>>,
        consequent: Box<Spanned<Expr>>,
        alternate: Option<Box<Spanned<Expr>>>,
    },
    /// `do { stmts; final_expr }`
    Do {
        body: Vec<Spanned<Statement>>,
        value: Box<Spanned<Expr>>,
    },
    /// `f(a, b, c)`
    Call {
        callee: Box<Spanned<Expr>>,
        args: Vec<Spanned<Expr>>,
    },
    /// `a[b]`
    SoftIndex {
        target: Box<Spanned<Expr>>,
        key: Box<Spanned<Expr>>,
    },
    /// `foo.bar`
    HardIndex {
        target: Box<Spanned<Expr>>,
        key: String,
    },
    /// `[a, b, c]`
    Array(Vec<Spanned<Expr>>),
    /// `{ [k1]: v1, [k2]: v2 }`
    Map(Vec<Spanned<MapEntry>>),
    /// `$'hello, ${name}'`
    FormatString(Vec<FormatSegment>),
    /// `fn name?(params) -> body`
    Lambda {
        /// Non-variadic parameters, excluding a `...rest` param.
        params: Vec<Spanned<Binding>>,
        /// Variadic param, preceded by `...`.
        variadic_param: Option<Spanned<Binding>>,
        /// Lambdas may or may not have a name for self-recursion.
        self_name: Option<Spanned<String>>,
        /// Non-tail statements.
        body: Vec<Spanned<Statement>>,
        /// Tail position expression which evaluates to the return value.
        return_expr: Box<Spanned<Expr>>,
    },
    AbbreviatedLambda {
        /// A single body expression that contains dollar identifiers.
        /// This is the only place dollar identifiers are legal (parser-enforced).
        body: Box<Spanned<Expr>>,
    },
    /// `filter structure with operation`
    Filter {
        structure: Box<Spanned<Expr>>,
        operation: Box<Spanned<Expr>>,
    },
    /// `map structure with operation`
    MapIter {
        structure: Box<Spanned<Expr>>,
        operation: Box<Spanned<Expr>>,
    },
    /// `reduce structure [init: init_expr] with operation`
    Reduce {
        structure: Box<Spanned<Expr>>,
        operation: Box<Spanned<Expr>>,
        init: Option<Box<Spanned<Expr>>>,
    },
    /// `foreach structure with operation`
    Foreach {
        structure: Box<Spanned<Expr>>,
        operation: Box<Spanned<Expr>>,
    },
    /// `match target { pattern => result, ... }`
    Match {
        target: Box<Spanned<Expr>>,
        arms: Vec<Spanned<MatchArm>>,
    },
}

// -- Literals --

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Vec<u8>),
}

// -- Operators --

#[derive(Clone, Copy, Debug, Serialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    And,
    Or,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum UnaryOp {
    Negate,
    Not,
}

// -- Map entries --

#[derive(Clone, Debug, Serialize)]
pub struct MapEntry {
    pub key: Spanned<Expr>,
    pub value: Spanned<Expr>,
}

// -- Format strings --

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum FormatSegment {
    Literal(Vec<u8>),
    Interpolation(Spanned<Expr>),
}

// -- Match --

#[derive(Clone, Debug, Serialize)]
pub struct MatchArm {
    pub pattern: Spanned<MatchPattern>,
    pub guard: Option<Spanned<Expr>>,
    pub result: Spanned<Expr>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum MatchPattern {
    /// `name` or `name is Type` or `_` or `_ is Type`.
    Binding {
        name: Spanned<Binding>,
        type_constraint: Option<Spanned<TypeConstraint>>,
    },
    /// `(expr)` or a literal: compare by value.
    Value(Spanned<Expr>),
    /// `[p1, p2, ...rest]`
    Array {
        elements: Vec<Spanned<MatchPattern>>,
        rest: Option<Spanned<Binding>>,
    },
    /// `{key: pattern, ...} as name?`
    Map {
        entries: Vec<Spanned<MapPatternEntry>>,
        bind_whole: Option<Spanned<Binding>>,
    },
    /// `p1 | p2 | p3`
    Alternative(Vec<Spanned<MatchPattern>>),
}

#[derive(Clone, Debug, Serialize)]
pub struct MapPatternEntry {
    pub key: Spanned<Expr>,
    pub pattern: Spanned<MatchPattern>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum TypeConstraint {
    Null,
    Int,
    Float,
    Bool,
    String,
    Array,
    Map,
    Function,
    Primitive,
    Numeric,
    Structured,
    Nonnull,
}

// -- Destructuring (for `def`) --

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Destructure {
    /// `def name = ...`
    Binding(Spanned<Binding>),
    /// `def [a, b, ...rest] = ...`
    Array {
        elements: Vec<Spanned<Destructure>>,
        rest: Option<Spanned<Binding>>,
    },
    /// `def {key: name, ...} as whole? = ...`
    Map {
        entries: Vec<Spanned<MapDestructureEntry>>,
        bind_whole: Option<Spanned<Binding>>,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct MapDestructureEntry {
    pub key: Spanned<Expr>,
    pub destructure: Spanned<Destructure>,
}
