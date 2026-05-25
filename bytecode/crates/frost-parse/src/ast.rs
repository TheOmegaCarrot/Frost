// -- Source location --

use std::ops::Range;

/// A range in source code, from start (inclusive) to end (exclusive).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

// -- Binding --

/// A name binding: either a named identifier or a discard (`_`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Binding {
    Named(String),
    Discarded,
}

// -- Program --

/// A program is a sequence of statements.
#[derive(Debug)]
pub struct Program {
    pub statements: Vec<Statement>,
}

// -- Statements --

#[derive(Debug)]
pub struct Statement {
    pub kind: StatementKind,
    pub span: SourceSpan,
}

#[derive(Debug)]
pub enum StatementKind {
    /// `def name = expr` or `export def name = expr`
    Def {
        exported: bool,
        destructure: Destructure,
        expr: Expr,
    },
    /// A bare expression executed for its side effects.
    Expr(Expr),
}

// -- Expressions --

#[derive(Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: SourceSpan,
}

#[derive(Debug)]
pub enum ExprKind {
    /// A literal value: `42`, `3.14`, `"hello"`, `true`, `null`.
    Literal(Literal),
    /// A variable reference: `foo`.
    NameLookup(String),
    /// A binary operation: `a + b`, `x == y`, `p and q`.
    BinOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    /// A unary operation: `-x`, `not x`.
    UnaryOp { op: UnaryOp, operand: Box<Expr> },
    /// `if cond: then elif cond2: then2 else: fallback`
    If {
        condition: Box<Expr>,
        consequent: Box<Expr>,
        alternate: Option<Box<Expr>>,
    },
    /// `do { stmts; final_expr }`
    Do {
        body: Vec<Statement>,
        value: Box<Expr>,
    },
    /// `f(a, b, c)`
    Call { callee: Box<Expr>, args: Vec<Expr> },
    /// `a[b]` or `a.foo`
    Index { target: Box<Expr>, key: Box<Expr> },
    /// `[a, b, c]`
    Array(Vec<Expr>),
    /// `{ [k1]: v1, [k2]: v2 }`
    Map(Vec<(Expr, Expr)>),
    /// `$'hello, ${name}'`
    FormatString(Vec<FormatSegment>),
    /// `fn name?(params) -> body`
    Lambda {
        /// Non-variadic parameters, excluding a `...rest` param.
        params: Vec<Binding>,
        /// Variadic param, preceded by `...`.
        variadic_param: Option<Binding>,
        /// Lambdas may or may not have a name for self-recursion.
        self_name: Option<String>,
        /// Non-tail statements.
        body: Vec<Statement>,
        /// Tail position expression which evaluates to the return value.
        return_expr: Box<Expr>,
    },
    /// `filter structure with operation`
    Filter {
        structure: Box<Expr>,
        operation: Box<Expr>,
    },
    /// `map structure with operation`
    MapIter {
        structure: Box<Expr>,
        operation: Box<Expr>,
    },
    /// `reduce structure [init: init_expr] with operation`
    Reduce {
        structure: Box<Expr>,
        operation: Box<Expr>,
        init: Option<Box<Expr>>,
    },
    /// `foreach structure with operation`
    Foreach {
        structure: Box<Expr>,
        operation: Box<Expr>,
    },
    /// `match target { pattern => result, ... }`
    Match {
        target: Box<Expr>,
        arms: Vec<MatchArm>,
    },
}

// -- Literals --

#[derive(Debug)]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Vec<u8>),
}

// -- Operators --

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub enum UnaryOp {
    Negate,
    Not,
}

// -- Format strings --

#[derive(Debug)]
pub enum FormatSegment {
    Literal(Vec<u8>),
    Interpolation(Expr),
}

// -- Match --

#[derive(Debug)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub guard: Option<Expr>,
    pub result: Expr,
}

#[derive(Debug)]
pub struct MatchPattern {
    pub kind: MatchPatternKind,
    pub span: SourceSpan,
}

#[derive(Debug)]
pub enum MatchPatternKind {
    /// `name` or `name is Type` or `_` or `_ is Type`.
    Binding {
        name: Binding,
        type_constraint: Option<TypeConstraint>,
    },
    /// `(expr)` or a literal -- compare by value.
    Value(Expr),
    /// `[p1, p2, ...rest]`
    Array {
        elements: Vec<MatchPattern>,
        rest: Option<Binding>,
    },
    /// `{key: pattern, ...} as name?`
    Map {
        entries: Vec<MapPatternEntry>,
        bind_whole: Option<Binding>,
    },
    /// `p1 | p2 | p3`
    Alternative(Vec<MatchPattern>),
}

#[derive(Debug)]
pub struct MapPatternEntry {
    pub key: Expr,
    pub pattern: MatchPattern,
}

#[derive(Clone, Copy, Debug)]
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

#[derive(Debug)]
pub struct Destructure {
    pub kind: DestructureKind,
    pub span: SourceSpan,
}

#[derive(Debug)]
pub enum DestructureKind {
    /// `def name = ...`
    Binding(Binding),
    /// `def [a, b, ...rest] = ...`
    Array {
        elements: Vec<Destructure>,
        rest: Option<Binding>,
    },
    /// `def {key: name, ...} as whole? = ...`
    Map {
        entries: Vec<MapDestructureEntry>,
        bind_whole: Option<Binding>,
    },
}

#[derive(Debug)]
pub struct MapDestructureEntry {
    pub key: Expr,
    pub destructure: Destructure,
}
