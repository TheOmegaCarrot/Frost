//! Drawing a [`Program`] as a tree, for a person inspecting what source
//! parses to.
//!
//! Each AST node converts to one [`Node`] by exhaustive matches, so a new kind
//! of AST node cannot be missed; drawing then works on that one shape alone.
//! A `Node`'s children are references back into the AST, converted only when
//! drawn, and drawing works from an explicit stack, so the tree draws in
//! constant native stack however deeply it nests.

use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};

use crate::ast::{
    BinOp, Binding, Expr, FormatSegment, Literal, LogicalOp, MapEntry, MapPatternEntry, MatchArm,
    Pattern, Program, SourceSpan, Spanned, Statement, UnaryOp,
};
use crate::line_index::LineIndex;

/// The most characters a leaf shows before it is cut short with an ellipsis.
/// The reader has the source at hand, so the start of a long name or literal
/// is enough to recognize it.
const MAX_LEAF_CHARS: usize = 40;

impl Program {
    /// A drawing of this tree, for a person inspecting what `source` parses
    /// to. `source` is the text the program was parsed from: each node shows
    /// where it starts there, as line and column.
    ///
    /// Its layout may change from one version to the next.
    pub fn display_tree<'a>(&'a self, source: &'a str) -> TreeDisplay<'a> {
        TreeDisplay {
            program: self,
            source,
            style: Style {
                unicode: false,
                color: false,
            },
        }
    }
}

/// A drawing of a [`Program`]'s tree, from [`Program::display_tree`], shown
/// through [`Display`].
#[derive(Clone, Copy, Debug)]
pub struct TreeDisplay<'a> {
    program: &'a Program,
    source: &'a str,
    style: Style,
}

impl TreeDisplay<'_> {
    /// Set whether the tree is drawn with Unicode box drawing rather than
    /// ASCII. Only Unicode shows non-ASCII text as itself; ASCII escapes it,
    /// so that the whole drawing is ASCII. It is ASCII by default.
    pub fn with_unicode(mut self, unicode: bool) -> Self {
        self.style.unicode = unicode;
        self
    }

    /// Set whether the tree is colored with ANSI escape codes, for a terminal.
    /// It is not, by default.
    pub fn with_color(mut self, color: bool) -> Self {
        self.style.color = color;
        self
    }
}

impl Display for TreeDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let style = self.style;
        let lines = draw(
            &self.program.statements,
            &LineIndex::new(self.source),
            style,
        );
        let gutter_width = lines
            .iter()
            .filter_map(|line| line.location.as_ref())
            .map(String::len)
            .max()
            .unwrap_or(0);
        for Line { location, text } in &lines {
            let (padding, gutter) = match location {
                Some(location) => (gutter_width - location.len(), style.dim(location)),
                None => (gutter_width, String::new()),
            };
            writeln!(f, "{:padding$}{gutter}  {text}", "")?;
        }
        Ok(())
    }
}

// -- The tree --

/// A part of the AST that draws as one node.
#[derive(Clone, Copy)]
enum Subtree<'a> {
    Statement(&'a Spanned<Statement>),
    Expr(&'a Spanned<Expr>),
    Pattern(&'a Spanned<Pattern>),
    MapEntry(&'a Spanned<MapEntry>),
    MapPatternEntry(&'a Spanned<MapPatternEntry>),
    MatchArm(&'a Spanned<MatchArm>),
    /// The literal text of a format String, between interpolations.
    FormatText(&'a str),
}

/// One node as drawn: its kind, its leaves inline after the kind, and its
/// children below.
struct Node<'a> {
    kind: &'static str,
    leaves: Vec<Leaf<'a>>,
    /// Where it starts, a byte offset; `None` for a node with no span of its own.
    start: Option<u32>,
    children: Vec<Child<'a>>,
}

/// What sits inline on a node's line: a name, literal, operator, or flag.
enum Leaf<'a> {
    /// Text shown as is, escaped only where it could break the drawing:
    /// names, operators, numbers, signatures, and flags.
    Code(Cow<'a, str>),
    /// A String's text, shown quoted and escaped.
    Text(&'a str),
    /// A Bytes literal's bytes, shown as a Bytes literal.
    Bytes(&'a [u8]),
}

/// One child below a node. A node's children are all unlabeled where they
/// fill one field of the AST node, and otherwise all labeled by field.
enum Child<'a> {
    /// A child node, labeled with its field if its parent labels its children.
    Single(Option<&'static str>, Subtree<'a>),
    /// A list field: its label's own line, then a child node per element.
    List(&'static str, Vec<Subtree<'a>>),
}

impl<'a> Node<'a> {
    /// A node of `kind` spanning `span`, with nothing inline and no children yet.
    fn new(kind: &'static str, span: SourceSpan) -> Self {
        Self {
            // Saturating, as `LineIndex` treats offsets past 4 GiB: inexactly.
            start: Some(u32::try_from(span.start).unwrap_or(u32::MAX)),
            ..Self::unplaced(kind)
        }
    }

    /// A node of `kind` with no span of its own.
    fn unplaced(kind: &'static str) -> Self {
        Self {
            kind,
            leaves: Vec::new(),
            start: None,
            children: Vec::new(),
        }
    }

    fn with_leaves(mut self, leaves: impl IntoIterator<Item = Leaf<'a>>) -> Self {
        self.leaves.extend(leaves);
        self
    }

    /// Give the node `children`, unlabeled.
    fn with_children(mut self, children: impl IntoIterator<Item = Subtree<'a>>) -> Self {
        self.children = children
            .into_iter()
            .map(|child| Child::Single(None, child))
            .collect();
        self
    }

    /// Give the node children labeled by field, leaving out each `None`: an
    /// absent optional field or an empty list.
    fn with_fields(mut self, fields: impl IntoIterator<Item = Option<Child<'a>>>) -> Self {
        self.children = fields.into_iter().flatten().collect();
        self
    }
}

impl<'a> Leaf<'a> {
    fn code(code: impl Into<Cow<'a, str>>) -> Self {
        Self::Code(code.into())
    }
}

/// The field `label` holding `child`.
fn one<'a>(label: &'static str, child: Subtree<'a>) -> Option<Child<'a>> {
    Some(Child::Single(Some(label), child))
}

/// The field `label` holding `child`, if it is present.
fn optional<'a>(label: &'static str, child: Option<Subtree<'a>>) -> Option<Child<'a>> {
    child.and_then(|child| one(label, child))
}

/// The list field `label` holding `children`, if there are any.
fn list<'a>(label: &'static str, children: Vec<Subtree<'a>>) -> Option<Child<'a>> {
    (!children.is_empty()).then_some(Child::List(label, children))
}

// -- From the AST --

impl<'a> Subtree<'a> {
    fn node(self) -> Node<'a> {
        match self {
            Self::Statement(statement) => statement_node(statement),
            Self::Expr(expr) => expr_node(expr),
            Self::Pattern(pattern) => pattern_node(pattern),
            Self::MapEntry(entry) => map_entry_node(entry),
            Self::MapPatternEntry(entry) => map_pattern_entry_node(entry),
            Self::MatchArm(arm) => match_arm_node(arm),
            Self::FormatText(text) => Node::unplaced("Text").with_leaves([Leaf::Text(text)]),
        }
    }
}

fn statement_node(statement: &Spanned<Statement>) -> Node<'_> {
    match &statement.node {
        Statement::Def {
            exported,
            pattern,
            expr,
        } => Node::new("Def", statement.span)
            .with_leaves(exported.then(|| Leaf::code("export")))
            .with_fields([
                one("pattern", Subtree::Pattern(pattern)),
                one("expr", Subtree::Expr(expr)),
            ]),
        // The statement adds nothing to its expression.
        Statement::Expr(expr) => expr_node(expr),
    }
}

fn statements(statements: &[Spanned<Statement>]) -> Vec<Subtree<'_>> {
    statements.iter().map(Subtree::Statement).collect()
}

fn exprs(exprs: &[Spanned<Expr>]) -> Vec<Subtree<'_>> {
    exprs.iter().map(Subtree::Expr).collect()
}

fn expr_node(expr: &Spanned<Expr>) -> Node<'_> {
    let span = expr.span;
    match &expr.node {
        Expr::Literal(literal) => Node::new("Literal", span).with_leaves([literal_leaf(literal)]),
        Expr::NameLookup(name) => Node::new("Name", span).with_leaves([Leaf::code(name)]),
        Expr::BinOp { left, op, right } => Node::new("BinOp", span)
            .with_leaves([Leaf::code(bin_op_symbol(op.node))])
            .with_fields([
                one("left", Subtree::Expr(left)),
                one("right", Subtree::Expr(right)),
            ]),
        Expr::Logical { left, op, right } => Node::new("Logical", span)
            .with_leaves([Leaf::code(logical_op_symbol(op.node))])
            .with_fields([
                one("left", Subtree::Expr(left)),
                one("right", Subtree::Expr(right)),
            ]),
        Expr::UnaryOp { op, operand } => Node::new("UnaryOp", span)
            .with_leaves([Leaf::code(unary_op_symbol(op.node))])
            .with_children([Subtree::Expr(operand)]),
        Expr::If {
            condition,
            consequent,
            alternate,
        } => Node::new("If", span).with_fields([
            one("condition", Subtree::Expr(condition)),
            one("consequent", Subtree::Expr(consequent)),
            optional("alternate", alternate.as_deref().map(Subtree::Expr)),
        ]),
        Expr::Do { body, value } => Node::new("Do", span).with_fields([
            list("body", statements(body)),
            one("value", Subtree::Expr(value)),
        ]),
        Expr::Call { callee, args } => Node::new("Call", span).with_fields([
            one("callee", Subtree::Expr(callee)),
            list("args", exprs(args)),
        ]),
        Expr::SoftIndex { target, key } => Node::new("SoftIndex", span).with_fields([
            one("target", Subtree::Expr(target)),
            one("key", Subtree::Expr(key)),
        ]),
        Expr::HardIndex { target, key } => Node::new("HardIndex", span)
            .with_leaves([Leaf::code(format!(".{}", key.node))])
            .with_children([Subtree::Expr(target)]),
        Expr::Array(elements) => Node::new("Array", span).with_children(exprs(elements)),
        Expr::Map(entries) => {
            Node::new("Map", span).with_children(entries.iter().map(Subtree::MapEntry))
        }
        Expr::FormatString(segments) => {
            Node::new("FormatString", span).with_children(segments.iter().map(|segment| {
                match segment {
                    FormatSegment::Literal(text) => Subtree::FormatText(text),
                    FormatSegment::Interpolation(expr) => Subtree::Expr(expr),
                }
            }))
        }
        Expr::Lambda {
            params,
            variadic_param,
            self_name,
            body,
            return_expr,
        } => Node::new("Lambda", span)
            .with_leaves([Leaf::code(lambda_signature(
                self_name.as_ref(),
                params,
                variadic_param.as_ref(),
            ))])
            .with_fields([
                list("body", statements(body)),
                one("return_expr", Subtree::Expr(return_expr)),
            ]),
        Expr::AbbreviatedLambda {
            used_params,
            uses_rest,
            body,
        } => Node::new("AbbreviatedLambda", span)
            .with_leaves([Leaf::code(abbreviated_lambda_signature(
                used_params,
                *uses_rest,
            ))])
            .with_children([Subtree::Expr(body)]),
        Expr::Match { target, arms } => Node::new("Match", span).with_fields([
            one("target", Subtree::Expr(target)),
            list("arms", arms.iter().map(Subtree::MatchArm).collect()),
        ]),
    }
}

/// A lambda's name and parameters as written: `f(x, ...rest)`, or `(x)`
/// for a lambda without a name.
fn lambda_signature(
    self_name: Option<&Spanned<String>>,
    params: &[Spanned<Binding>],
    variadic_param: Option<&Spanned<Binding>>,
) -> String {
    let params = params
        .iter()
        .map(|param| binding_text(&param.node).to_owned())
        .chain(variadic_param.map(|rest| format!("...{}", binding_text(&rest.node))));
    let name = self_name.map_or("", |name| name.node.as_str());
    format!("{name}({})", params.collect::<Vec<_>>().join(", "))
}

/// The parameters an abbreviated lambda implies, as if written out:
/// `($1, _, $3, $$)`. An unused positional parameter shows as `_`, as if
/// discarded.
fn abbreviated_lambda_signature(used_params: &[bool], uses_rest: bool) -> String {
    let params = used_params
        .iter()
        .zip(1..)
        .map(|(&used, n)| {
            if used {
                format!("${n}")
            } else {
                "_".to_owned()
            }
        })
        .chain(uses_rest.then(|| "$$".to_owned()));
    format!("({})", params.collect::<Vec<_>>().join(", "))
}

fn literal_leaf(literal: &Literal) -> Leaf<'_> {
    match literal {
        Literal::Null => Leaf::code("null"),
        Literal::Bool(b) => Leaf::code(if *b { "true" } else { "false" }),
        Literal::Int(n) => Leaf::code(n.to_string()),
        // Debug, unlike Display, keeps the point of a whole Float: `1.0`, not `1`.
        Literal::Float(x) => Leaf::code(format!("{x:?}")),
        Literal::String(text) => Leaf::Text(text),
        Literal::Bytes(bytes) => Leaf::Bytes(bytes),
    }
}

fn map_entry_node(entry: &Spanned<MapEntry>) -> Node<'_> {
    let MapEntry { key, value } = &entry.node;
    Node::new("MapEntry", entry.span).with_fields([
        one("key", Subtree::Expr(key)),
        one("value", Subtree::Expr(value)),
    ])
}

fn match_arm_node(arm: &Spanned<MatchArm>) -> Node<'_> {
    let MatchArm {
        pattern,
        guard,
        result,
    } = &arm.node;
    Node::new("MatchArm", arm.span).with_fields([
        one("pattern", Subtree::Pattern(pattern)),
        optional("guard", guard.as_ref().map(Subtree::Expr)),
        one("result", Subtree::Expr(result)),
    ])
}

fn pattern_node(pattern: &Spanned<Pattern>) -> Node<'_> {
    let span = pattern.span;
    match &pattern.node {
        Pattern::Binding {
            name,
            type_constraint,
        } => Node::new("BindingPattern", span).with_leaves(
            [
                Some(Leaf::code(binding_text(&name.node))),
                type_constraint
                    .as_ref()
                    .map(|constraint| Leaf::code(format!("is {}", constraint.node.name()))),
            ]
            .into_iter()
            .flatten(),
        ),
        Pattern::Value(expr) => {
            Node::new("ValuePattern", span).with_children([Subtree::Expr(expr)])
        }
        Pattern::Array {
            elements,
            rest,
            bind_whole,
        } => Node::new("ArrayPattern", span)
            .with_leaves(
                [
                    rest.as_ref()
                        .map(|rest| Leaf::code(format!("...{}", binding_text(&rest.node)))),
                    bind_whole.as_ref().map(as_leaf),
                ]
                .into_iter()
                .flatten(),
            )
            .with_children(elements.iter().map(Subtree::Pattern)),
        Pattern::Map {
            entries,
            bind_whole,
        } => Node::new("MapPattern", span)
            .with_leaves(bind_whole.as_ref().map(as_leaf))
            .with_children(entries.iter().map(Subtree::MapPatternEntry)),
        Pattern::Alternative(alternatives) => Node::new("AlternativePattern", span)
            .with_children(alternatives.iter().map(Subtree::Pattern)),
    }
}

fn map_pattern_entry_node(entry: &Spanned<MapPatternEntry>) -> Node<'_> {
    let MapPatternEntry {
        key,
        optional,
        pattern,
    } = &entry.node;
    Node::new("MapPatternEntry", entry.span)
        .with_leaves(optional.then(|| Leaf::code("optional")))
        .with_fields([
            one("key", Subtree::Expr(key)),
            one("pattern", Subtree::Pattern(pattern)),
        ])
}

/// The leaf for an `as` binding of a whole Array or Map.
fn as_leaf(binding: &Spanned<Binding>) -> Leaf<'static> {
    Leaf::code(format!("as {}", binding_text(&binding.node)))
}

fn binding_text(binding: &Binding) -> &str {
    match binding {
        Binding::Named(name) => name,
        Binding::Discarded => "_",
    }
}

fn bin_op_symbol(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::Neq => "!=",
        BinOp::Lt => "<",
        BinOp::Lte => "<=",
        BinOp::Gt => ">",
        BinOp::Gte => ">=",
    }
}

fn logical_op_symbol(op: LogicalOp) -> &'static str {
    match op {
        LogicalOp::And => "and",
        LogicalOp::Or => "or",
    }
}

fn unary_op_symbol(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Negate => "-",
        UnaryOp::Not => "not",
    }
}

// -- Drawing --

/// One line of the drawing: the location shown in the gutter, as
/// `line:column`, if the line is a node's, and the rest of the line, already
/// styled.
struct Line {
    location: Option<String>,
    text: String,
}

/// A child still to draw: the child, the start of its line, and the start of
/// each line below it.
struct PendingChild<'a> {
    child: Child<'a>,
    lead: String,
    indent: String,
}

/// The lines drawing `roots` and everything below them, top to bottom.
fn draw(roots: &[Spanned<Statement>], index: &LineIndex, style: Style) -> Vec<Line> {
    let glyphs = style.glyphs();
    let mut lines = Vec::new();
    // Children are pushed in reverse, so that each pops in the order it is drawn.
    let mut pending: Vec<PendingChild<'_>> = roots
        .iter()
        .rev()
        .map(|root| PendingChild {
            child: Child::Single(None, Subtree::Statement(root)),
            lead: String::new(),
            indent: String::new(),
        })
        .collect();
    while let Some(PendingChild {
        child,
        lead,
        indent,
    }) = pending.pop()
    {
        let mut text = lead;
        let (location, children) = match child {
            Child::Single(label, subtree) => {
                let node = subtree.node();
                if let Some(label) = label {
                    text.push_str(&style.label(&format!("{label}:")));
                    text.push(' ');
                }
                text.push_str(&style.kind(node.kind));
                for leaf in &node.leaves {
                    text.push(' ');
                    text.push_str(&style.leaf(leaf));
                }
                let location = node.start.map(|start| index.location(start).to_string());
                (location, node.children)
            }
            Child::List(label, elements) => {
                text.push_str(&style.label(label));
                let elements = elements.into_iter().map(|e| Child::Single(None, e));
                (None, elements.collect())
            }
        };
        lines.push(Line { location, text });
        let count = children.len();
        for (i, child) in children.into_iter().enumerate().rev() {
            let (branch, continuation) = if i + 1 == count {
                (glyphs.last_branch, glyphs.blank)
            } else {
                (glyphs.branch, glyphs.trunk)
            };
            pending.push(PendingChild {
                child,
                lead: style.dim(&format!("{indent}{branch}")),
                indent: format!("{indent}{continuation}"),
            });
        }
    }
    lines
}

/// The characters that draw the tree.
struct Glyphs {
    /// Leads to a child with a sibling below it.
    branch: &'static str,
    /// Leads to a last child.
    last_branch: &'static str,
    /// Continues a parent's line past a child with a sibling below it.
    trunk: &'static str,
    /// Stands in for a parent's line past its last child.
    blank: &'static str,
    /// Ends a leaf cut short.
    ellipsis: &'static str,
}

const ASCII: Glyphs = Glyphs {
    branch: "|-- ",
    last_branch: "`-- ",
    trunk: "|   ",
    blank: "    ",
    ellipsis: "...",
};

const UNICODE: Glyphs = Glyphs {
    branch: "\u{251c}\u{2500}\u{2500} ",
    last_branch: "\u{2514}\u{2500}\u{2500} ",
    trunk: "\u{2502}   ",
    blank: "    ",
    ellipsis: "\u{2026}",
};

/// How the tree is drawn: its characters, and whether it is colored with
/// ANSI escape codes.
#[derive(Clone, Copy, Debug)]
struct Style {
    unicode: bool,
    color: bool,
}

impl Style {
    fn glyphs(self) -> &'static Glyphs {
        if self.unicode { &UNICODE } else { &ASCII }
    }

    fn kind(self, text: &str) -> String {
        self.paint(text, "36")
    }

    fn label(self, text: &str) -> String {
        self.paint(text, "33")
    }

    /// Text that structures the drawing rather than saying what parsed.
    fn dim(self, text: &str) -> String {
        self.paint(text, "90")
    }

    fn paint(self, text: &str, code: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_owned()
        }
    }

    /// `leaf` as shown, cut short if it is long.
    fn leaf(self, leaf: &Leaf<'_>) -> String {
        match leaf {
            Leaf::Code(code) => self.cut_short(self.escaped(code, false)),
            Leaf::Text(text) => format!("\"{}\"", self.cut_short(self.escaped(text, true))),
            Leaf::Bytes(bytes) => {
                let hex = bytes.iter().map(|byte| format!("{byte:02x}").into());
                format!("x'{}'", self.cut_short(hex))
            }
        }
    }

    /// `pieces` joined, cut short with an ellipsis past [`MAX_LEAF_CHARS`].
    /// Each piece stands for one character of a leaf, such as an escape, and
    /// is never cut in two.
    fn cut_short<'p>(self, pieces: impl Iterator<Item = Cow<'p, str>>) -> String {
        let mut shown = String::new();
        let mut shown_chars = 0;
        for piece in pieces {
            shown_chars += piece.chars().count();
            if shown_chars > MAX_LEAF_CHARS {
                shown.push_str(&self.dim(self.glyphs().ellipsis));
                break;
            }
            shown.push_str(&piece);
        }
        shown
    }

    /// `text`'s characters escaped as Frost displays a String, except that
    /// ASCII drawing also escapes non-ASCII text, and only `quoted` text
    /// escapes `"`.
    ///
    /// A parsed program's names are ASCII identifiers, which escape as
    /// themselves; escaping them anyway keeps a program built by hand from
    /// breaking the drawing.
    fn escaped(self, text: &str, quoted: bool) -> impl Iterator<Item = Cow<'static, str>> {
        text.chars().map(move |c| match c {
            '"' if quoted => r#"\""#.into(),
            '\\' => r"\\".into(),
            '\n' => r"\n".into(),
            '\t' => r"\t".into(),
            '\r' => r"\r".into(),
            c if c.is_control() || (!self.unicode && !c.is_ascii()) => {
                format!(r"\u{{{:x}}}", u32::from(c)).into()
            }
            c => c.to_string().into(),
        })
    }
}
