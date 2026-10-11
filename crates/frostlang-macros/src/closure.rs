//! `frost_closure!`: a Frost function calling a Rust closure.

use std::mem;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Attribute, Expr, ExprClosure, Ident, Pat, PatType, ReturnType, Token};

use crate::errors::Errors;
use crate::inputs::{Inputs, ctx_type, value_type};
use crate::private;

/// The macro's input: the Frost function's name, then the closure.
struct Input {
    name: Expr,
    closure: ExprClosure,
}

impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name = input.parse()?;
        if input.is_empty() {
            return Err(syn::Error::new(
                Span::call_site(),
                "`frost_closure!` needs the Frost function's name, then the closure: \
                 `frost_closure!(\"...\", |...| ...)`",
            ));
        }
        input.parse::<Token![,]>()?;
        let closure = match input.parse()? {
            Expr::Closure(closure) => closure,
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "`frost_closure!` takes a closure; for a function, see `#[frost_fn]`",
                ));
            }
        };
        input.parse::<Option<Token![,]>>()?;
        Ok(Self { name, closure })
    }
}

/// The Frost function calling the closure, which is kept as written, less its
/// markers, with a type given to each parameter written without one; or, for
/// an invalid closure, its errors.
pub(crate) fn expand(input: TokenStream) -> TokenStream {
    let Input { name, mut closure } = match syn::parse2(input) {
        Ok(input) => input,
        Err(error) => return failed(error),
    };
    let mut errors = Errors::default();
    check_closure(&closure, &mut errors);
    let inputs = read_inputs(&mut closure, &mut errors);
    match errors.finish() {
        Ok(()) => function(&name, &closure, &inputs),
        Err(error) => failed(error),
    }
}

/// `error`, in an expression that is still a `Value`, so that the errors are
/// all the caller sees.
fn failed(error: syn::Error) -> TokenStream {
    let error = error.into_compile_error();
    let frostlang = private(Span::call_site());
    quote!({ #error #frostlang::Value::Null })
}

/// Records an error for each way `closure` is not a plain closure.
fn check_closure(closure: &ExprClosure, errors: &mut Errors) {
    let not = |thing: &str| format!("`frost_closure!` takes a plain closure, not {thing}");
    errors.keep(closure.modifiers.require_empty());
    if let Some(lifetimes) = &closure.lifetimes {
        errors.push(syn::Error::new_spanned(
            lifetimes,
            not("one with a `for<...>` binder"),
        ));
    }
    if let Some(constness) = &closure.constness {
        errors.push(syn::Error::new_spanned(constness, not("a `const` one")));
    }
    if let Some(asyncness) = &closure.asyncness {
        errors.push(syn::Error::new_spanned(asyncness, not("an `async` one")));
    }
}

/// Reads the inputs from the closure's parameters, stripping their markers,
/// and giving each parameter written without a type the one it is taken as:
/// so the closure's body type-checks before the Frost function calls it.
fn read_inputs(closure: &mut ExprClosure, errors: &mut Errors) -> Inputs {
    let mut inputs = Inputs::default();
    for param in &mut closure.inputs {
        if let Pat::Type(typed) = param {
            inputs.read(&mut typed.attrs, &typed.pat, Some(&typed.ty), errors);
            continue;
        }
        // Without a type, a parameter's attributes are its pattern's.
        let mut attrs = take_attrs(param);
        let ty = if inputs.read(&mut attrs, param, None, errors) {
            ctx_type()
        } else {
            value_type()
        };
        let pat = mem::replace(param, Pat::Verbatim(TokenStream::new()));
        *param = Pat::Type(PatType {
            attrs,
            colon_token: Token![:](pat.span()),
            pat: Box::new(pat),
            ty: Box::new(ty),
        });
    }
    inputs
}

/// The attributes of an untyped parameter's pattern, taken out of it.
fn take_attrs(pat: &mut Pat) -> Vec<Attribute> {
    match pat {
        Pat::Ident(ident) => mem::take(&mut ident.attrs),
        Pat::Wild(wild) => mem::take(&mut wild.attrs),
        // Any other pattern is rejected, so its attributes need no reading.
        _ => Vec::new(),
    }
}

/// The Frost function named `name`, calling `closure`.
fn function(name: &Expr, closure: &ExprClosure, inputs: &Inputs) -> TokenStream {
    let frostlang = private(Span::call_site());
    // Mixed-site, so that neither the closure nor what it captures sees them.
    let [name_local, params, function, ctx, args] =
        ["name", "params", "function", "ctx", "args"].map(|n| Ident::new(n, Span::mixed_site()));
    let (ctx_pattern, ctx_arg) = inputs.ctx_pattern_and_arg(&ctx);
    let spec = inputs.spec(closure.inputs_begin.span);
    let takes = inputs.takes(&args);
    // A result that cannot be returned is reported at its type, or else at
    // the body.
    let result_span = match &closure.output {
        ReturnType::Type(_, ty) => ty.span(),
        ReturnType::Default => closure.body.span(),
    };
    // See `frost_fn` for why it is not mixed-site.
    let result = Ident::new("result", result_span);
    let result_frostlang = private(result_span);
    let convert = quote_spanned!(result_span=>
        #result_frostlang::IntoNativeResult::into_result(#result, #name_local)
    );

    // Requires of the closure, where it is written, what `checked_native` will
    // of the function calling it: so rustc reports a closure falling short at
    // the closure, such as at what it mutates, not at the generated code. The
    // helper is declared in a block of its own, out of the closure's sight.
    let arg_types: Vec<Ident> = (0..inputs.args.len())
        .map(|i| format_ident!("A{i}"))
        .collect();
    let ctx_type = inputs.ctx.then(|| quote!(&mut #frostlang::NativeCtx<'_>,));
    let required = quote! {{
        fn required<F, R, #(#arg_types),*>(function: F) -> F
        where
            F: Fn(#ctx_type #(#arg_types),*) -> R + Send + Sync + 'static,
        {
            function
        }
        required
    }};

    quote! {
        {
            // In the order written, so that the name may use what the closure
            // then moves.
            let #name_local: &'static str = #name;
            let #function = (#required)(#closure);
            let #params = const { #spec };
            #frostlang::Value::checked_native(
                #name_local,
                #params.clone(),
                move |#ctx_pattern, #args| {
                    let mut #args = #frostlang::Args::new(#name_local, #params.clone(), #args);
                    let #result = #function(#ctx_arg #(#takes),*);
                    #convert
                },
            )
        }
    }
}
