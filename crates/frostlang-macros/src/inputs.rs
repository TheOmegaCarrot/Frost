//! A Frost function's inputs, read from the parameters of the Rust function or
//! closure it calls: an optional leading context, then its arguments.

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, quote_spanned};
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::{Attribute, Ident, Pat, PatIdent, Type};

use crate::errors::Errors;
use crate::private;

/// The inputs of a Frost function.
pub(crate) struct Inputs {
    /// Whether the first parameter is the context, a `&mut NativeCtx`.
    pub(crate) ctx: bool,
    pub(crate) args: Vec<Arg>,
}

/// A parameter taken from the Frost function's arguments.
pub(crate) struct Arg {
    /// Its name in the spec, which errors report.
    name: String,
    ty: Type,
}

impl Inputs {
    /// Reads each parameter in order, stripping its `#[frost(...)]` markers
    /// from `attrs`, and recording every error in `errors`.
    pub(crate) fn read<'a>(
        params: impl IntoIterator<Item = (&'a mut Vec<Attribute>, &'a Pat, &'a Type)>,
        errors: &mut Errors,
    ) -> Self {
        let mut inputs = Self {
            ctx: false,
            args: Vec::new(),
        };
        for (index, (attrs, pat, ty)) in params.into_iter().enumerate() {
            if let Some(ctx) = errors.keep(take_markers(attrs)).flatten() {
                if inputs.ctx {
                    errors.push(syn::Error::new(
                        ctx,
                        "only one parameter can be `#[frost(ctx)]`",
                    ));
                } else if index > 0 {
                    errors.push(syn::Error::new(
                        ctx,
                        "the `#[frost(ctx)]` parameter must come first",
                    ));
                } else {
                    inputs.ctx = true;
                }
                continue;
            }
            if let Some(impl_token) = find_impl(ty.to_token_stream()) {
                errors.push(syn::Error::new(
                    impl_token,
                    "a Frost function's parameter type cannot be `impl Trait`: \
                     the macro must name it",
                ));
            }
            if let Some(name) = errors.keep(arg_name(pat)) {
                inputs.args.push(Arg {
                    name,
                    ty: ty.clone(),
                });
            }
        }
        inputs
    }

    /// The `Params` spec, built in const evaluation, so that a spec breaking
    /// its rules fails to compile with the error spanning `span`.
    pub(crate) fn spec(&self, span: Span) -> TokenStream {
        let frostlang = private(span);
        let params = self.args.iter().map(Arg::param);
        quote_spanned!(span=> #frostlang::Params::new(&[#(#params),*]))
    }

    /// An expression per argument, in order, taking it from the `Args` named
    /// `args`.
    pub(crate) fn takes<'a>(&'a self, args: &'a Ident) -> impl Iterator<Item = TokenStream> + 'a {
        self.args.iter().map(move |arg| {
            let ty = &arg.ty;
            quote_spanned!(ty.span()=> #args.take::<#ty>()?)
        })
    }
}

impl Arg {
    fn param(&self) -> TokenStream {
        let (ty, name) = (&self.ty, &self.name);
        let frostlang = private(ty.span());
        quote_spanned!(ty.span()=> <#ty as #frostlang::FrostArg>::PARAM.named(#name))
    }
}

/// Strips the `#[frost(...)]` markers from `attrs`, returning the span of a
/// `ctx` marker if there is one.
fn take_markers(attrs: &mut Vec<Attribute>) -> syn::Result<Option<Span>> {
    let (markers, others): (Vec<_>, Vec<_>) = attrs
        .drain(..)
        .partition(|attr| attr.path().is_ident("frost"));
    *attrs = others;

    let mut errors = Errors::default();
    let mut ctx = None;
    for marker in markers {
        let read = marker.parse_nested_meta(|meta| {
            if !meta.path.is_ident("ctx") {
                return Err(meta.error("unknown marker: a parameter takes only `#[frost(ctx)]`"));
            }
            if !meta.input.is_empty() && !meta.input.peek(syn::Token![,]) {
                return Err(meta.error("`ctx` takes no value"));
            }
            if ctx.is_some() {
                return Err(meta.error("only one parameter can be `#[frost(ctx)]`"));
            }
            ctx = Some(meta.path.span());
            Ok(())
        });
        errors.keep(read);
    }
    errors.finish().map(|()| ctx)
}

/// The name `pat` gives its argument: a plain identifier, less any `r#`.
fn arg_name(pat: &Pat) -> syn::Result<String> {
    match pat {
        Pat::Ident(PatIdent {
            by_ref: None,
            subpat: None,
            ident,
            ..
        }) => Ok(ident.unraw().to_string()),
        _ => Err(syn::Error::new_spanned(
            pat,
            "a Frost function's parameter must be a plain name, such as `text`",
        )),
    }
}

/// The span of an `impl` keyword anywhere in `tokens`, which in a type can
/// only begin an `impl Trait`.
fn find_impl(tokens: TokenStream) -> Option<Span> {
    tokens.into_iter().find_map(|token| match token {
        TokenTree::Ident(ident) if ident == "impl" => Some(ident.span()),
        TokenTree::Group(group) => find_impl(group.stream()),
        _ => None,
    })
}
