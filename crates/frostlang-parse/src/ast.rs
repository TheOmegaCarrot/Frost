// -- Source location --

use std::ops::Range;

use serde::Serialize;

/// A byte range in the source, from `start` (inclusive) to `end` (exclusive).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SourceSpan {
    /// Inclusive start offset.
    pub start: usize,
    /// Exclusive end offset.
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

impl From<SourceSpan> for Range<usize> {
    /// The span as a range, for slicing the source: `&source[Range::from(span)]`.
    fn from(value: SourceSpan) -> Self {
        value.start..value.end
    }
}

// -- Spanned --

/// Pairs an AST payload with its source span.
/// Its span obeys the [crate-level span invariants](crate#span-invariants).
///
/// # Equality ignores spans
///
/// `==` compares the `node` only: two trees that parse the same modulo
/// whitespace are equal, even though their spans differ.
/// Spans are provenance, not content.
/// Compare `.span` explicitly where position matters.
#[derive(Clone, Debug, Serialize)]
pub struct Spanned<T> {
    /// The AST payload.
    pub node: T,
    /// Where `node` appears in the source.
    pub span: SourceSpan,
}

// Hash must not be derived for Spanned: it would hash the span and disagree
// with this span-ignoring PartialEq.
impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl<T> Spanned<T> {
    /// Pairs `node` with `span`.
    pub fn new(node: T, span: SourceSpan) -> Self {
        Self { node, span }
    }
}

// -- Binding --

/// A name binding: either a named identifier or a discard (`_`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Binding {
    /// Binds the given name.
    Named(String),
    /// `_`: binds nothing.
    Discarded,
}

// -- Program --

/// A program is a sequence of statements.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Program {
    /// Top-level statements, in source order.
    pub statements: Vec<Spanned<Statement>>,
}

impl Program {
    /// The tree as text, for a person inspecting what source parses to. Its
    /// layout may change from one version to the next.
    pub fn dump(&self) -> String {
        // TODO: A more compact printer; spans make this verbose.
        format!("{:#?}", self.statements)
    }
}

// -- Statements --

/// A statement: a definition or an expression.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Statement {
    /// `def name = expr` or `export def name = expr`
    ///
    /// `defn name(params) -> body` (optionally `export`ed) also produces this variant,
    /// as `def name = fn name(params) -> body`.
    Def {
        /// Whether the definition is prefixed with `export`.
        exported: bool,
        /// The binding target left of `=`.
        destructure: Spanned<Destructure>,
        /// The value right of `=`.
        expr: Spanned<Expr>,
    },
    /// A bare expression executed for its side effects.
    Expr(Spanned<Expr>),
}

// -- Expressions --

/// An expression.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Expr {
    /// A literal value: `42`, `3.14`, `"hello"`, `true`, `null`.
    Literal(Literal),
    /// A variable reference: `foo`.
    NameLookup(String),
    /// A binary operation: `a + b`, `x == y`.
    BinOp {
        /// Left operand.
        left: Box<Spanned<Expr>>,
        /// The operator.
        op: Spanned<BinOp>,
        /// Right operand.
        right: Box<Spanned<Expr>>,
    },
    /// A short-circuiting logical operation: `p and q`, `p or q`.
    ///
    /// Separate from [`Expr::BinOp`] because the right operand is evaluated conditionally.
    Logical {
        /// Left operand.
        left: Box<Spanned<Expr>>,
        /// The operator.
        op: Spanned<LogicalOp>,
        /// Right operand.
        right: Box<Spanned<Expr>>,
    },
    /// A unary operation: `-x`, `not x`.
    UnaryOp {
        /// The operator.
        op: Spanned<UnaryOp>,
        /// The operand.
        operand: Box<Spanned<Expr>>,
    },
    /// `if cond: then elif cond2: then2 else: fallback`
    If {
        /// The condition.
        condition: Box<Spanned<Expr>>,
        /// The branch taken when `condition` is truthy.
        consequent: Box<Spanned<Expr>>,
        /// The `else` branch, or a nested [`Expr::If`] for `elif`; `None` when omitted.
        alternate: Option<Box<Spanned<Expr>>>,
    },
    /// `do { stmts; final_expr }`
    Do {
        /// Statements before the final expression.
        body: Vec<Spanned<Statement>>,
        /// The final expression, whose value is the block's value.
        value: Box<Spanned<Expr>>,
    },
    /// `f(a, b, c)`
    ///
    /// The threaded form `a @ f(b, c)` also produces this variant, with `a` as the first argument.
    Call {
        /// The called expression.
        callee: Box<Spanned<Expr>>,
        /// The arguments, in order.
        args: Vec<Spanned<Expr>>,
    },
    /// `a[b]`
    SoftIndex {
        /// The indexed expression.
        target: Box<Spanned<Expr>>,
        /// The key inside `[]`.
        key: Box<Spanned<Expr>>,
    },
    /// `foo.bar`
    HardIndex {
        /// The indexed expression.
        target: Box<Spanned<Expr>>,
        /// The field name after `.`.
        key: Spanned<String>,
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
    /// `$(expr)`: an abbreviated lambda.
    ///
    /// The parameter list is implied by the dollar identifiers the body uses;
    /// the parser summarizes them here so a consumer never parses `$n` names.
    AbbreviatedLambda {
        /// `used_params[i]` is whether the positional `$n` with n = i + 1 is referenced in the body
        /// (`$` counts as `$1`).
        /// The length is the parameter count: the highest positional referenced.
        /// Empty for the zero-arg thunk form,
        /// and the last entry of a non-empty list is always true.
        used_params: Vec<bool>,
        /// Whether the rest parameter `$$` is referenced.
        uses_rest: bool,
        /// A single body expression that contains dollar identifiers, kept
        /// verbatim (`$` is not normalized to `$1`).
        /// This is the only place dollar identifiers are legal (parser-enforced).
        body: Box<Spanned<Expr>>,
    },
    /// `filter structure with operation`
    Filter {
        /// The collection before `with`.
        structure: Box<Spanned<Expr>>,
        /// The function after `with`.
        operation: Box<Spanned<Expr>>,
    },
    /// `map structure with operation`
    MapIter {
        /// The collection before `with`.
        structure: Box<Spanned<Expr>>,
        /// The function after `with`.
        operation: Box<Spanned<Expr>>,
    },
    /// `reduce structure [init: init_expr] with operation`
    Reduce {
        /// The collection before `with`.
        structure: Box<Spanned<Expr>>,
        /// The function after `with`.
        operation: Box<Spanned<Expr>>,
        /// The `init:` value; `None` when omitted.
        init: Option<Box<Spanned<Expr>>>,
    },
    /// `foreach structure with operation`
    Foreach {
        /// The collection before `with`.
        structure: Box<Spanned<Expr>>,
        /// The function after `with`.
        operation: Box<Spanned<Expr>>,
    },
    /// `match target { pattern => result, ... }`
    Match {
        /// The value being matched.
        target: Box<Spanned<Expr>>,
        /// The arms, in source order.
        arms: Vec<Spanned<MatchArm>>,
    },
}

// -- Literals --

/// A literal value; see [`Expr::Literal`].
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Literal {
    /// `null`
    Null,
    /// `true` or `false`
    Bool(bool),
    /// An Int literal.
    Int(i64),
    /// A Float literal.
    Float(f64),
    /// A String literal, with escapes resolved.
    String(String),
    /// A Bytes literal, decoded.
    Bytes(Vec<u8>),
}

// -- Operators --

/// The binary operators; see [`Expr::BinOp`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum BinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Mod,
    /// `==`
    Eq,
    /// `!=`
    Neq,
    /// `<`
    Lt,
    /// `<=`
    Lte,
    /// `>`
    Gt,
    /// `>=`
    Gte,
}

/// The short-circuiting logical operators; see [`Expr::Logical`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum LogicalOp {
    /// `and`
    And,
    /// `or`
    Or,
}

/// The unary operators; see [`Expr::UnaryOp`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum UnaryOp {
    /// `-`
    Negate,
    /// `not`
    Not,
}

// -- Map entries --

/// One `key: value` entry of an [`Expr::Map`].
///
/// The shorthand `name` is represented as the String key `"name"`
/// with an [`Expr::NameLookup`] of `name`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MapEntry {
    /// The key; a bare `name` key is a String literal.
    pub key: Spanned<Expr>,
    /// The value.
    pub value: Spanned<Expr>,
}

// -- Format strings --

/// A piece of an [`Expr::FormatString`].
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum FormatSegment {
    /// Literal text, with escapes resolved.
    Literal(String),
    /// A `${expr}` interpolation.
    Interpolation(Spanned<Expr>),
}

// -- Match --

/// One `pattern => result` arm of an [`Expr::Match`].
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MatchArm {
    /// The pattern.
    pub pattern: Spanned<MatchPattern>,
    /// The `if:` guard; `None` when omitted.
    pub guard: Option<Spanned<Expr>>,
    /// The arm's value when it matches.
    pub result: Spanned<Expr>,
}

/// A pattern in a [`MatchArm`].
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum MatchPattern {
    /// `name` or `name is Type` or `_` or `_ is Type`.
    Binding {
        /// The bound name, or `_`.
        name: Spanned<Binding>,
        /// The type after `is`; `None` when omitted.
        type_constraint: Option<Spanned<TypeConstraint>>,
    },
    /// `(expr)` or a literal: compare by value.
    Value(Spanned<Expr>),
    /// `[p1, p2, ...rest]`
    Array {
        /// The element patterns, before any rest.
        elements: Vec<Spanned<MatchPattern>>,
        /// The `...rest` binding; `None` when omitted.
        rest: Option<Spanned<Binding>>,
    },
    /// `{key: pattern, ...} as name?`
    Map {
        /// The entries.
        entries: Vec<Spanned<MapPatternEntry>>,
        /// The `as` binding for the whole Map; `None` when omitted.
        bind_whole: Option<Spanned<Binding>>,
    },
    /// `p1 | p2 | p3`
    Alternative(Vec<Spanned<MatchPattern>>),
}

/// One `key: pattern` entry of a [`MatchPattern::Map`].
///
/// The shorthand `name` (or `name is Type`) is represented as the String key `"name"`
/// with a [`MatchPattern::Binding`] of `name`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MapPatternEntry {
    /// The key; a bare `name` key is a String literal.
    pub key: Spanned<Expr>,
    /// The pattern the entry's value must match.
    pub pattern: Spanned<MatchPattern>,
}

/// The type named after `is` in a [`MatchPattern::Binding`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TypeConstraint {
    /// `is Null`
    Null,
    /// `is Int`
    Int,
    /// `is Float`
    Float,
    /// `is Bool`
    Bool,
    /// `is String`
    String,
    /// `is Bytes`
    Bytes,
    /// `is Array`
    Array,
    /// `is Map`
    Map,
    /// `is Function`
    Function,
    /// `is Opaque`
    Opaque,
    /// `is Primitive`
    Primitive,
    /// `is Numeric`
    Numeric,
    /// `is Structured`
    Structured,
    /// `is Flat`
    Flat,
    /// `is Nonnull`
    Nonnull,
}

// -- Destructuring (for `def`) --

/// The binding target of a [`Statement::Def`].
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum Destructure {
    /// `def name = ...`
    Binding(Spanned<Binding>),
    /// `def [a, b, ...rest] = ...`
    Array {
        /// The element targets, before any rest.
        elements: Vec<Spanned<Destructure>>,
        /// The `...rest` binding; `None` when omitted.
        rest: Option<Spanned<Binding>>,
    },
    /// `def {key: name, ...} as whole? = ...`
    Map {
        /// The entries.
        entries: Vec<Spanned<MapDestructureEntry>>,
        /// The `as` binding for the whole Map; `None` when omitted.
        bind_whole: Option<Spanned<Binding>>,
    },
}

/// One `key: target` entry of a [`Destructure::Map`].
///
/// The shorthand `name` is represented as the String key `"name"`
/// with a [`Destructure::Binding`] of `name`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MapDestructureEntry {
    /// The key; a bare `name` key is a String literal.
    pub key: Spanned<Expr>,
    /// The target the entry's value binds to.
    pub destructure: Spanned<Destructure>,
}
