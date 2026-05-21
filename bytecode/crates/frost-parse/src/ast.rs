/// A program is a sequence of statements.
pub struct Program {
    pub statements: Vec<Statement>,
}

// -- Statements --

pub enum Statement {
    /// `def name = expr` or `export def name = expr`
    Def {
        destructure: Destructure,
        value: Expr,
        exported: bool,
    },
    /// A bare expression executed for its side effects.
    Expr(Expr),
}

// -- Expressions --

pub enum Expr {
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
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
    },
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
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    /// `a[b]` or `a.foo`
    Index {
        target: Box<Expr>,
        key: Box<Expr>,
    },
    /// `[a, b, c]`
    Array(Vec<Expr>),
    /// `{ [k1]: v1, [k2]: v2 }`
    Map(Vec<(Expr, Expr)>),
    /// `$'hello, ${name}'`
    FormatString(Vec<FormatSegment>),
    /// `fn name?(params) -> body`
    Lambda(Lambda),
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

pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(Vec<u8>),
}

// -- Operators --

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
pub enum UnaryOp {
    Negate,
    Not,
}

// -- Format strings --

pub enum FormatSegment {
    Literal(Vec<u8>),
    Interpolation(Expr),
}

// -- Lambdas --

pub struct Lambda {
    pub params: Vec<String>,
    pub variadic: Option<String>,
    pub self_name: Option<String>,
    pub body: Vec<Statement>,
    pub return_expr: Box<Expr>,
}

// -- Match --

pub struct MatchArm {
    pub pattern: MatchPattern,
    pub guard: Option<Expr>,
    pub result: Expr,
}

pub enum MatchPattern {
    /// `name` or `name is Type` or `_` or `_ is Type`.
    Binding {
        name: Option<String>,
        type_constraint: Option<TypeConstraint>,
    },
    /// `(expr)` or a literal — compare by value.
    Value(Expr),
    /// `[p1, p2, ...rest]`
    Array {
        elements: Vec<MatchPattern>,
        rest: Option<Option<String>>,
    },
    /// `{key: pattern, ...} as name?`
    Map {
        entries: Vec<MapPatternEntry>,
        bind_whole: Option<String>,
    },
    /// `p1 | p2 | p3`
    Alternative(Vec<MatchPattern>),
}

pub struct MapPatternEntry {
    pub key: Expr,
    pub pattern: MatchPattern,
}

#[derive(Clone, Copy)]
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

pub enum Destructure {
    /// `def name = ...`
    Binding(Option<String>),
    /// `def [a, b, ...rest] = ...`
    Array {
        elements: Vec<Destructure>,
        rest: Option<Option<String>>,
    },
    /// `def {key: name, ...} as whole? = ...`
    Map {
        entries: Vec<MapDestructureEntry>,
        bind_whole: Option<String>,
    },
}

pub struct MapDestructureEntry {
    pub key: Expr,
    pub destructure: Destructure,
}
