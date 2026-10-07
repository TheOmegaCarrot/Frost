#![allow(clippy::result_large_err)]

use std::borrow::Cow;

use crate::Arity;
use crate::bytecode::Bytecode;
use frostlang_parse::ast::{Binding, Expr, SourceSpan, Spanned, Statement};

use crate::compile::{
    Diagnostics,
    lower::{
        ExprFragment, FunctionBuilder, Ir, Position,
        fold::constant_of,
        locals::{LocalInfo, LocalKind},
        prewalk,
    },
};

/// The name given to a lambda the script leaves unnamed. It is not an
/// identifier, so no name in a script can refer to it.
const ANONYMOUS_NAME: &str = "<lambda>";

/// A lambda of either form, reduced to what compiling it needs.
struct NormalizedLambda<'a> {
    /// Every function has a name: the script's, or [`ANONYMOUS_NAME`].
    name: &'a str,
    params: Vec<Param<'a>>,
    variadic_param: Option<Param<'a>>,
    /// The binding for the closure itself: the name, for recursion, or a
    /// discard for an anonymous lambda.
    self_binding: Param<'a>,
    body: &'a [Spanned<Statement>],
    return_expr: &'a Spanned<Expr>,
}

/// A parameter binding, borrowed from the script or generated.
type Param<'a> = Spanned<Cow<'a, Binding>>;

/// A parameter the compiler generates, which has no span of its own in the
/// source.
fn generated(binding: Binding) -> Param<'static> {
    Spanned {
        node: Cow::Owned(binding),
        span: SourceSpan::default(),
    }
}

/// A parameter written in the script.
fn written(binding: &Spanned<Binding>) -> Param<'_> {
    Spanned {
        node: Cow::Borrowed(&binding.node),
        span: binding.span,
    }
}

impl<'a> NormalizedLambda<'a> {
    /// `lambda` must be an [`Expr::Lambda`] or [`Expr::AbbreviatedLambda`].
    fn new(lambda: &'a Expr) -> Self {
        match lambda {
            Expr::Lambda {
                params,
                variadic_param,
                self_name,
                body,
                return_expr,
            } => Self {
                name: self_name.as_ref().map_or(ANONYMOUS_NAME, |name| &name.node),
                params: params.iter().map(written).collect(),
                variadic_param: variadic_param.as_ref().map(written),
                self_binding: match self_name {
                    Some(name) => Spanned {
                        node: Cow::Owned(Binding::Named(name.node.clone())),
                        span: name.span,
                    },
                    None => generated(Binding::Discarded),
                },
                body,
                return_expr,
            },
            // The positional parameters are `$1`, `$2`, ...; one the body never
            // uses is discarded. The rest parameter is `$$`.
            Expr::AbbreviatedLambda {
                used_params,
                uses_rest,
                body,
            } => Self {
                name: ANONYMOUS_NAME,
                params: used_params
                    .iter()
                    .enumerate()
                    .map(|(index, &used)| {
                        generated(if used {
                            Binding::Named(format!("${}", index + 1))
                        } else {
                            Binding::Discarded
                        })
                    })
                    .collect(),
                variadic_param: uses_rest.then(|| generated(Binding::Named("$$".to_string()))),
                self_binding: generated(Binding::Discarded),
                body: &[],
                return_expr: body,
            },
            _ => unreachable!("NormalizedLambda::new called on a non-lambda expression"),
        }
    }

    fn arity(&self) -> Arity {
        let fixed = self.params.len();
        match self.variadic_param {
            Some(_) => Arity::AtLeast(fixed),
            None => Arity::Exact(fixed),
        }
    }
}

impl FunctionBuilder<'_> {
    /// `fn name(a, b, ...rest) -> { body; return_expr }` or `$(body)`.
    ///
    /// `lambda` must be an [`Expr::Lambda`] or [`Expr::AbbreviatedLambda`].
    pub(super) fn compile_lambda(
        &mut self,
        lambda: &Spanned<Expr>,
    ) -> Result<ExprFragment, Diagnostics> {
        // A free name this function binds is a capture, seated in the order
        // `free_names` finds them; the order is arbitrary, but both sides of the
        // closure must agree on it. Any other free name is the child's to
        // resolve: a global, or an unbound-name error.
        let free_names: Vec<String> = prewalk::free_names(&lambda.node)
            .into_iter()
            .filter(|name| self.locals.resolve(name).is_some())
            .collect();

        // The push that seats each capture, in the same order the child seats
        // them. A captured constant is pushed directly, so a lambda over
        // constants can still be evaluated in a fold; hoisted, it is built into
        // the child instead and not pushed at all. A known value that cannot be
        // a constant is pushed from its local, and known to the child's folds.
        let hoist = self.options.optimization_options.capture_hoist;
        let mut captures = Vec::new();
        let mut capture_pushes = Vec::new();
        let mut hoisted = Vec::new();
        for name in free_names {
            let push = self.compile_name_lookup(&name, SourceSpan::default())?;
            match constant_of(&push.code) {
                Some(value) if hoist => hoisted.push((name, value)),
                _ => {
                    let known = match push.code.as_slice() {
                        [Ir::LoadLocal(id)] => self.locals.constant(*id).cloned(),
                        _ => None,
                    };
                    captures.push((name, known));
                    capture_pushes.push(push);
                }
            }
        }

        let lambda = NormalizedLambda::new(&lambda.node);
        let mut child = self.child(lambda.name.to_owned(), lambda.arity(), captures, hoisted);

        // The call leaves `[closure, arg1 .. argN, rest?]` on the stack, which is
        // also source order. Define the names in that order, so a duplicate is
        // reported at its second appearance, then store from the top down.
        let slots = [&lambda.self_binding]
            .into_iter()
            .chain(&lambda.params)
            .chain(&lambda.variadic_param)
            .map(|param| match param.node.as_ref() {
                Binding::Discarded => Ok(None),
                Binding::Named(name) => child
                    .locals
                    .define(LocalInfo {
                        name: name.clone(),
                        span: param.span,
                        exported: false,
                        constant: None,
                        kind: LocalKind::Param,
                    })
                    .map(Some)
                    .map_err(|original| self.duplicate_binding(name, param.span, original)),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let prelude = slots.into_iter().rev().map(|slot| match slot {
            Some(id) => Ir::DefLocal(id),
            None => Ir::Ready(Bytecode::Pop),
        });

        let compiled_body = lambda
            .body
            .iter()
            .map(|stmt| child.compile_statement(stmt, Position::Inner))
            .collect::<Result<Vec<_>, _>>()?;

        // The return expression is the root of the body's expression tree, so it
        // is a fold point, as a tail statement is.
        let compiled_tail = child.compile_expression(lambda.return_expr, Position::Tail)?;
        let compiled_tail = child.fold_if_eligible(compiled_tail);

        let child_ir: Vec<Ir> = prelude
            .into_iter()
            .chain(compiled_body.into_iter().flat_map(|stmt| stmt.code))
            .chain(compiled_tail.code)
            .collect();

        let lowered = child.finish(child_ir);

        self.effectful |= lowered.effectful;

        // Creating the closure is safe in a fold if calling it is (it has no
        // effects) and every captured value is known now.
        let foldable = !lowered.effectful && capture_pushes.iter().all(|push| push.foldable);

        Ok(ExprFragment {
            foldable,
            code: capture_pushes
                .into_iter()
                .flat_map(|push| push.code)
                .chain([Ir::closure(lowered)])
                .collect(),
        })
    }
}
