//! A Frost function's inputs, read from the parameters of the Rust function or
//! closure it calls: an optional leading context, then its arguments.

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, quote, quote_spanned};
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::{Attribute, Ident, Pat, PatIdent, Type};

use crate::errors::Errors;
use crate::private;

/// The inputs of a Frost function, read one parameter at a time.
#[derive(Default)]
pub(crate) struct Inputs {
    /// The span of the context parameter's type, if the first parameter is
    /// the context; a context of the wrong type is reported there.
    pub(crate) ctx: Option<Span>,
    pub(crate) args: Vec<Arg>,
    /// How many parameters have been read.
    read: usize,
}

/// A parameter taken from the Frost function's arguments.
pub(crate) struct Arg {
    /// Its name in the spec, which errors report; none when marked
    /// `#[frost(unnamed)]`.
    name: Option<String>,
    ty: Type,
}

/// The markers on one parameter.
#[derive(Default)]
struct Markers {
    /// The span of a `ctx` marker.
    ctx: Option<Span>,
    /// The span of an `unnamed` marker.
    unnamed: Option<Span>,
}

impl Inputs {
    /// Reads the next parameter, `pat` of type `ty`, stripping its
    /// `#[frost(...)]` markers from `attrs`, and recording every error in
    /// `errors`. An argument without a type is a `Value`, accepting Any.
    ///
    /// Returns whether it is marked as the context.
    pub(crate) fn read(
        &mut self,
        attrs: &mut Vec<Attribute>,
        pat: &Pat,
        ty: Option<&Type>,
        errors: &mut Errors,
    ) -> bool {
        let index = self.read;
        self.read += 1;
        let markers = errors.keep(take_markers(attrs)).unwrap_or_default();
        if let Some(ctx) = markers.ctx {
            if let Some(unnamed) = markers.unnamed {
                errors.push(syn::Error::new(
                    unnamed,
                    "the context is no argument, so has no name to leave out",
                ));
            }
            if self.ctx.is_some() {
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
                self.ctx = Some(ty.map_or_else(|| pat.span(), Spanned::span));
            }
            return true;
        }
        let ty = ty.cloned().unwrap_or_else(value_type);
        if let Some(impl_token) = find_impl(ty.to_token_stream()) {
            errors.push(syn::Error::new(
                impl_token,
                "a Frost function's parameter type cannot be `impl Trait`: \
                 the macro must name it",
            ));
        }
        // An unnamed argument's pattern need not give a name.
        if markers.unnamed.is_some() {
            self.args.push(Arg { name: None, ty });
        } else if let Some(name) = errors.keep(arg_name(pat)) {
            self.args.push(Arg {
                name: Some(name),
                ty,
            });
        }
        false
    }

    /// The `Params` spec, to be evaluated in a const context, so that a spec
    /// breaking its rules fails to compile with the error spanning `span`.
    pub(crate) fn spec(&self, span: Span) -> TokenStream {
        let frostlang = private(span);
        let params = self.args.iter().map(Arg::param);
        // The slice is a const of its own to be `'static`: an inline const
        // block, unlike a const item, extends no temporary it passes to a call.
        quote_spanned!(span=> #frostlang::Params::new(const { &[#(#params),*] }))
    }

    /// The pattern binding the `NativeCtx` of a call as `ctx`, and the
    /// statement and argument passing it on, as [`pass_ctx`](Self::pass_ctx).
    pub(crate) fn ctx_pattern_and_pass(
        &self,
        ctx: &Ident,
    ) -> (TokenStream, (TokenStream, TokenStream)) {
        let pattern = match self.ctx {
            Some(_) => quote!(mut #ctx),
            None => quote!(_),
        };
        (pattern, self.pass_ctx(quote!(&mut #ctx)))
    }

    /// A statement binding `ctx`, an expression of the context to pass on, to
    /// a local, and that local as the first argument of a call; nothing
    /// without a context parameter.
    ///
    /// The local is spanned at the parameter's type, so that rustc reports a
    /// context of the wrong type there, not at the macro. As in
    /// `result::conversion`, it is not mixed-site, whose hygiene would break
    /// that span, and nothing can shadow it.
    pub(crate) fn pass_ctx(&self, ctx: TokenStream) -> (TokenStream, TokenStream) {
        let Some(span) = self.ctx else {
            return (TokenStream::new(), TokenStream::new());
        };
        let local = Ident::new("context", span);
        (quote!(let #local = #ctx;), quote!(#local,))
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
        let ty = &self.ty;
        let frostlang = private(ty.span());
        let named = self.name.as_ref().map(|name| quote!(.named(#name)));
        quote_spanned!(ty.span()=> <#ty as #frostlang::FrostArg>::PARAM #named)
    }
}

/// The type of an argument written without one.
pub(crate) fn value_type() -> Type {
    let frostlang = private(Span::call_site());
    syn::parse_quote!(#frostlang::Value)
}

/// The type of the context parameter.
pub(crate) fn ctx_type() -> Type {
    let frostlang = private(Span::call_site());
    syn::parse_quote!(&mut #frostlang::NativeCtx<'_>)
}

/// Strips the `#[frost(...)]` markers from `attrs`, returning them.
fn take_markers(attrs: &mut Vec<Attribute>) -> syn::Result<Markers> {
    let (frost_attrs, others): (Vec<_>, Vec<_>) = attrs
        .drain(..)
        .partition(|attr| attr.path().is_ident("frost"));
    *attrs = others;

    let mut errors = Errors::default();
    let mut markers = Markers::default();
    for attr in frost_attrs {
        let read = attr.parse_nested_meta(|meta| {
            let (marker, slot) = if meta.path.is_ident("ctx") {
                ("ctx", &mut markers.ctx)
            } else if meta.path.is_ident("unnamed") {
                ("unnamed", &mut markers.unnamed)
            } else {
                return Err(meta.error(
                    "unknown marker: a parameter takes `#[frost(ctx)]` or `#[frost(unnamed)]`",
                ));
            };
            if !meta.input.is_empty() && !meta.input.peek(syn::Token![,]) {
                return Err(meta.error(format!("`{marker}` takes no value")));
            }
            if slot.is_some() {
                return Err(meta.error(format!("`{marker}` is given twice")));
            }
            *slot = Some(meta.path.span());
            Ok(())
        });
        errors.keep(read);
    }
    errors.finish().map(|()| markers)
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
