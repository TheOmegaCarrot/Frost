//! Walks a parsed Frost AST into the display-tree JSON the astviz template
//! consumes: `{ id, label, range?, wrapper?, children? }` nodes.
//!
//! Ranges are 1-based line/col and half-open, matching the parser's half-open
//! byte spans (the end column is exclusive).

use frost_parse::ast::*;
use serde::Serialize;

// -- Output shapes --

#[derive(Serialize)]
pub struct Range {
    pub sl: usize,
    pub sc: usize,
    pub el: usize,
    pub ec: usize,
}

#[derive(Serialize)]
pub struct Node {
    pub id: u32,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
    #[serde(skip_serializing_if = "is_false")]
    pub wrapper: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Node>,
}

#[derive(Serialize)]
pub struct Highlight {
    pub s: usize,
    pub e: usize,
    pub c: String,
}

fn is_false(b: &bool) -> bool {
    !*b
}

// -- Byte offset -> line:col --

/// Resolves half-open byte offsets to 1-based line/column positions. Columns
/// are byte offsets within the line (matching the template's 1-byte-per-column
/// rendering); Frost source is ASCII outside string literals.
pub struct LineIndex {
    line_starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    fn locate(&self, byte: usize) -> (usize, usize) {
        let line = self.line_starts.partition_point(|&start| start <= byte) - 1;
        let col = byte - self.line_starts[line] + 1;
        (line + 1, col)
    }

    fn range_of(&self, span: SourceSpan) -> Range {
        let (sl, sc) = self.locate(span.start);
        let (el, ec) = self.locate(span.end);
        Range { sl, sc, el, ec }
    }
}

// -- Walker --

pub struct Walker<'a> {
    index: &'a LineIndex,
    next_id: u32,
}

impl<'a> Walker<'a> {
    pub fn new(index: &'a LineIndex) -> Self {
        Self { index, next_id: 0 }
    }

    fn alloc(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// A real (non-wrapper) node carrying a source range.
    fn ranged(&mut self, label: String, span: SourceSpan, children: Vec<Node>) -> Node {
        let range = self.index.range_of(span);
        Node {
            id: self.alloc(),
            label,
            range: Some(range),
            wrapper: false,
            children,
        }
    }

    /// A structural wrapper node (italic, no range) grouping one or more children.
    fn wrap(&mut self, label: &str, children: Vec<Node>) -> Node {
        Node {
            id: self.alloc(),
            label: label.to_string(),
            range: None,
            wrapper: true,
            children,
        }
    }

    /// A leaf with no source range, used only where the AST genuinely carries
    /// no span (e.g. format-string literal segments). Rendered with a warning.
    fn no_range(&mut self, label: String) -> Node {
        Node {
            id: self.alloc(),
            label,
            range: None,
            wrapper: false,
            children: Vec::new(),
        }
    }

    // -- Statements --

    pub fn stmt(&mut self, s: &Spanned<Statement>) -> Node {
        match &s.node {
            Statement::Def {
                exported,
                destructure,
                expr,
            } => {
                let label = if *exported { "Def (export)" } else { "Def" };
                let binding = self.destructure(destructure);
                let binding = self.wrap("binding", vec![binding]);
                let value = self.expr(expr);
                let value = self.wrap("value", vec![value]);
                self.ranged(label.to_string(), s.span, vec![binding, value])
            }
            // A bare expression statement renders as the expression itself.
            Statement::Expr(expr) => self.expr(expr),
        }
    }

    // -- Expressions --

    pub fn expr(&mut self, e: &Spanned<Expr>) -> Node {
        let span = e.span;
        match &e.node {
            Expr::Literal(lit) => {
                self.ranged(format!("Literal {}", literal_label(lit)), span, vec![])
            }
            Expr::NameLookup(name) => self.ranged(format!("NameLookup {name}"), span, vec![]),
            Expr::BinOp { left, op, right } => {
                let l = self.expr(left);
                let l = self.wrap("left", vec![l]);
                let r = self.expr(right);
                let r = self.wrap("right", vec![r]);
                self.ranged(format!("BinOp {}", binop_symbol(op.node)), span, vec![l, r])
            }
            Expr::Logical { left, op, right } => {
                let l = self.expr(left);
                let l = self.wrap("left", vec![l]);
                let r = self.expr(right);
                let r = self.wrap("right", vec![r]);
                self.ranged(
                    format!("Logical {}", logical_symbol(op.node)),
                    span,
                    vec![l, r],
                )
            }
            Expr::UnaryOp { op, operand } => {
                let child = self.expr(operand);
                self.ranged(
                    format!("UnaryOp {}", unaryop_symbol(op.node)),
                    span,
                    vec![child],
                )
            }
            Expr::If {
                condition,
                consequent,
                alternate,
            } => {
                let mut children = vec![];
                let c = self.expr(condition);
                children.push(self.wrap("condition", vec![c]));
                let t = self.expr(consequent);
                children.push(self.wrap("consequent", vec![t]));
                if let Some(alt) = alternate {
                    let a = self.expr(alt);
                    children.push(self.wrap("alternate", vec![a]));
                }
                self.ranged("If".to_string(), span, children)
            }
            Expr::Do { body, value } => {
                let mut children: Vec<Node> = body.iter().map(|st| self.stmt(st)).collect();
                let v = self.expr(value);
                children.push(self.wrap("value", vec![v]));
                self.ranged("Do".to_string(), span, children)
            }
            Expr::Call { callee, args } => {
                let c = self.expr(callee);
                let mut children = vec![self.wrap("callee", vec![c])];
                for arg in args {
                    children.push(self.expr(arg));
                }
                self.ranged("Call".to_string(), span, children)
            }
            Expr::SoftIndex { target, key } => {
                let t = self.expr(target);
                let t = self.wrap("target", vec![t]);
                let k = self.expr(key);
                let k = self.wrap("key", vec![k]);
                self.ranged("SoftIndex".to_string(), span, vec![t, k])
            }
            Expr::HardIndex { target, key } => {
                let t = self.expr(target);
                let k = self.ranged(format!(".{}", key.node), key.span, vec![]);
                let k = self.wrap("key", vec![k]);
                self.ranged(format!("HardIndex .{}", key.node), span, vec![t, k])
            }
            Expr::Array(elems) => {
                let children = elems.iter().map(|el| self.expr(el)).collect();
                self.ranged("Array".to_string(), span, children)
            }
            Expr::Map(entries) => {
                let mut children = vec![];
                for ent in entries {
                    let k = self.expr(&ent.node.key);
                    let k = self.wrap("key", vec![k]);
                    let v = self.expr(&ent.node.value);
                    let v = self.wrap("value", vec![v]);
                    children.push(self.wrap("entry", vec![k, v]));
                }
                self.ranged("Map".to_string(), span, children)
            }
            Expr::FormatString(segments) => {
                let mut children = vec![];
                for seg in segments {
                    match seg {
                        FormatSegment::Literal(text) => {
                            children.push(self.no_range(format!("FmtLiteral {text:?}")));
                        }
                        FormatSegment::Interpolation(expr) => {
                            let inner = self.expr(expr);
                            children.push(self.wrap("interpolation", vec![inner]));
                        }
                    }
                }
                self.ranged("FormatString".to_string(), span, children)
            }
            Expr::Lambda {
                params,
                variadic_param,
                self_name,
                body,
                return_expr,
            } => {
                let label = lambda_label(self_name.as_ref(), params, variadic_param.as_ref());
                let mut children: Vec<Node> = body.iter().map(|st| self.stmt(st)).collect();
                let ret = self.expr(return_expr);
                children.push(self.wrap("return", vec![ret]));
                self.ranged(label, span, children)
            }
            Expr::AbbreviatedLambda {
                used_params,
                uses_rest,
                body,
            } => {
                let mut params: Vec<String> = used_params
                    .iter()
                    .enumerate()
                    .map(|(i, used)| {
                        if *used {
                            format!("${}", i + 1)
                        } else {
                            "_".to_string()
                        }
                    })
                    .collect();
                if *uses_rest {
                    params.push("...$$".to_string());
                }
                let signature = params.join(", ");
                let inner = self.expr(body);
                self.ranged(
                    format!("AbbreviatedLambda ({signature})"),
                    span,
                    vec![inner],
                )
            }
            Expr::Filter {
                structure,
                operation,
            } => self.iter_expr("Filter", span, structure, operation, None),
            Expr::MapIter {
                structure,
                operation,
            } => self.iter_expr("MapIter", span, structure, operation, None),
            Expr::Foreach {
                structure,
                operation,
            } => self.iter_expr("Foreach", span, structure, operation, None),
            Expr::Reduce {
                structure,
                operation,
                init,
            } => self.iter_expr("Reduce", span, structure, operation, init.as_deref()),
            Expr::Match { target, arms } => {
                let t = self.expr(target);
                let mut children = vec![self.wrap("target", vec![t])];
                for arm in arms {
                    let mut arm_children = vec![self.pattern(&arm.node.pattern)];
                    if let Some(guard) = &arm.node.guard {
                        let g = self.expr(guard);
                        arm_children.push(self.wrap("guard", vec![g]));
                    }
                    let res = self.expr(&arm.node.result);
                    arm_children.push(self.wrap("result", vec![res]));
                    children.push(self.wrap("arm", arm_children));
                }
                self.ranged("Match".to_string(), span, children)
            }
        }
    }

    fn iter_expr(
        &mut self,
        label: &str,
        span: SourceSpan,
        structure: &Spanned<Expr>,
        operation: &Spanned<Expr>,
        init: Option<&Spanned<Expr>>,
    ) -> Node {
        let s = self.expr(structure);
        let mut children = vec![self.wrap("structure", vec![s])];
        if let Some(init) = init {
            let i = self.expr(init);
            children.push(self.wrap("init", vec![i]));
        }
        let o = self.expr(operation);
        children.push(self.wrap("operation", vec![o]));
        self.ranged(label.to_string(), span, children)
    }

    // -- Match patterns --

    fn pattern(&mut self, p: &Spanned<MatchPattern>) -> Node {
        match &p.node {
            MatchPattern::Binding {
                name,
                type_constraint,
            } => {
                let mut label = format!("Binding {}", binding_str(name));
                if let Some(tc) = type_constraint {
                    label.push_str(&format!(" is {tc:?}"));
                }
                self.ranged(label, p.span, vec![])
            }
            MatchPattern::Value(expr) => {
                let inner = self.expr(expr);
                self.ranged("Value".to_string(), p.span, vec![inner])
            }
            MatchPattern::Array { elements, rest } => {
                let children = elements.iter().map(|el| self.pattern(el)).collect();
                let mut label = "Array".to_string();
                if let Some(rest) = rest {
                    label.push_str(&format!(" ...{}", binding_str(rest)));
                }
                self.ranged(label, p.span, children)
            }
            MatchPattern::Map {
                entries,
                bind_whole,
            } => {
                let mut children = vec![];
                for ent in entries {
                    let k = self.expr(&ent.node.key);
                    let k = self.wrap("key", vec![k]);
                    let pat = self.pattern(&ent.node.pattern);
                    children.push(self.wrap("entry", vec![k, pat]));
                }
                let mut label = "Map".to_string();
                if let Some(whole) = bind_whole {
                    label.push_str(&format!(" as {}", binding_str(whole)));
                }
                self.ranged(label, p.span, children)
            }
            MatchPattern::Alternative(patterns) => {
                let children = patterns.iter().map(|pat| self.pattern(pat)).collect();
                self.ranged("Alternative".to_string(), p.span, children)
            }
        }
    }

    // -- Destructure patterns (def) --

    fn destructure(&mut self, d: &Spanned<Destructure>) -> Node {
        match &d.node {
            Destructure::Binding(b) => {
                self.ranged(format!("Binding {}", binding_str(b)), d.span, vec![])
            }
            Destructure::Array { elements, rest } => {
                let children = elements.iter().map(|el| self.destructure(el)).collect();
                let mut label = "Array".to_string();
                if let Some(rest) = rest {
                    label.push_str(&format!(" ...{}", binding_str(rest)));
                }
                self.ranged(label, d.span, children)
            }
            Destructure::Map {
                entries,
                bind_whole,
            } => {
                let mut children = vec![];
                for ent in entries {
                    let k = self.expr(&ent.node.key);
                    let k = self.wrap("key", vec![k]);
                    let inner = self.destructure(&ent.node.destructure);
                    children.push(self.wrap("entry", vec![k, inner]));
                }
                let mut label = "Map".to_string();
                if let Some(whole) = bind_whole {
                    label.push_str(&format!(" as {}", binding_str(whole)));
                }
                self.ranged(label, d.span, children)
            }
        }
    }
}

// -- Label helpers --

fn binding_str(b: &Spanned<Binding>) -> String {
    match &b.node {
        Binding::Named(name) => name.clone(),
        Binding::Discarded => "_".to_string(),
    }
}

fn lambda_label(
    self_name: Option<&Spanned<String>>,
    params: &[Spanned<Binding>],
    variadic: Option<&Spanned<Binding>>,
) -> String {
    let mut parts: Vec<String> = params.iter().map(binding_str).collect();
    if let Some(v) = variadic {
        parts.push(format!("...{}", binding_str(v)));
    }
    match self_name {
        Some(name) => format!("Lambda {}({})", name.node, parts.join(", ")),
        None => format!("Lambda({})", parts.join(", ")),
    }
}

fn literal_label(lit: &Literal) -> String {
    match lit {
        Literal::Null => "Null".to_string(),
        Literal::Bool(b) => format!("Bool({b})"),
        Literal::Int(i) => format!("Int({i})"),
        Literal::Float(f) => format!("Float({f})"),
        Literal::String(text) => format!("String({text:?})"),
        Literal::Bytes(bytes) => {
            let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            format!("Bytes(x'{hex}')")
        }
    }
}

fn binop_symbol(op: BinOp) -> &'static str {
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

fn logical_symbol(op: LogicalOp) -> &'static str {
    match op {
        LogicalOp::And => "and",
        LogicalOp::Or => "or",
    }
}

fn unaryop_symbol(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Negate => "-",
        UnaryOp::Not => "not",
    }
}
