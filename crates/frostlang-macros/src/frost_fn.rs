//! `#[frost_fn]`: a Frost function calling a Rust function.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::ext::IdentExt;
use syn::parse::Parser;
use syn::spanned::Spanned;
use syn::{Expr, FnArg, Ident, ItemFn, ReturnType, Safety, Signature};

use crate::errors::Errors;
use crate::inputs::Inputs;
use crate::private;

/// The function as written, less its markers, and its maker; or, for an
/// invalid declaration, its errors, still with the function so that its
/// callers raise no further errors.
pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut function: ItemFn = match syn::parse2(item) {
        Ok(function) => function,
        Err(error) => return error.into_compile_error(),
    };
    let mut errors = Errors::default();
    let name = errors.keep(frost_name(attr));
    check_signature(&function, &mut errors);
    let inputs = read_inputs(&mut function.sig, &mut errors);
    match (errors.finish(), name) {
        (Ok(()), Some(name)) => {
            let maker = maker(&function, &name, &inputs);
            quote!(#function #maker)
        }
        (result, _) => {
            let error = result.err().map(syn::Error::into_compile_error);
            quote!(#error #function)
        }
    }
}

/// The Frost function's name, from the attribute's `name = ...`.
fn frost_name(attr: TokenStream) -> syn::Result<Expr> {
    let mut name = None;
    let parser = syn::meta::parser(|meta| {
        if !meta.path.is_ident("name") {
            return Err(meta.error("unknown argument: `#[frost_fn]` takes only `name`"));
        }
        if name.is_some() {
            return Err(meta.error("`name` is given twice"));
        }
        name = Some(meta.value()?.parse()?);
        Ok(())
    });
    parser.parse2(attr)?;
    name.ok_or_else(|| {
        syn::Error::new(
            Span::call_site(),
            "`#[frost_fn]` needs the Frost function's name: `#[frost_fn(name = \"...\")]`",
        )
    })
}

/// Records an error for each way `function` is not a plain function.
fn check_signature(function: &ItemFn, errors: &mut Errors) {
    let sig = &function.sig;
    let not = |thing: &str| format!("`#[frost_fn]` takes a plain function, not {thing}");
    errors.keep(function.modifiers.require_empty());
    if let Some(asyncness) = &sig.asyncness {
        errors.push(syn::Error::new_spanned(asyncness, not("an `async` one")));
    }
    if !matches!(sig.safety, Safety::Default) {
        errors.push(syn::Error::new_spanned(&sig.safety, not("an `unsafe` one")));
    }
    if let Some(abi) = &sig.abi {
        errors.push(syn::Error::new_spanned(abi, not("an `extern` one")));
    }
    let generic = not("a generic one: the macro must name each parameter's type");
    if !sig.generics.params.is_empty() {
        errors.push(syn::Error::new_spanned(&sig.generics, &generic));
    } else if let Some(where_clause) = &sig.generics.where_clause {
        errors.push(syn::Error::new_spanned(where_clause, &generic));
    }
    if let Some(variadic) = &sig.variadic {
        errors.push(syn::Error::new_spanned(variadic, not("a variadic one")));
    }
}

/// Reads the inputs from `sig`'s parameters, stripping their markers.
fn read_inputs(sig: &mut Signature, errors: &mut Errors) -> Inputs {
    let mut inputs = Inputs::default();
    for input in &mut sig.inputs {
        match input {
            FnArg::Typed(typed) => {
                inputs.read(&mut typed.attrs, &typed.pat, Some(&typed.ty), errors);
            }
            FnArg::Receiver(receiver) => errors.push(syn::Error::new_spanned(
                receiver,
                "`#[frost_fn]` takes a free function, not a method",
            )),
        }
    }
    inputs
}

/// `<function>_frost`, returning the Frost function named `name`.
fn maker(function: &ItemFn, name: &Expr, inputs: &Inputs) -> TokenStream {
    let frostlang = private(Span::call_site());
    let (vis, sig) = (&function.vis, &function.sig);
    let target = &sig.ident;
    let maker = format_ident!("{}_frost", target.unraw());
    let doc = format!("The Frost function made from [`{target}`].");
    // The maker exists exactly when the function does.
    let cfgs = function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"));

    // Mixed-site, so that no parameter name can shadow them.
    let ctx = Ident::new("ctx", Span::mixed_site());
    let args = Ident::new("args", Span::mixed_site());
    let (ctx_pattern, ctx_arg) = inputs.ctx_pattern_and_arg(&ctx);
    let spec = inputs.spec(sig.paren_token.span.join());
    let takes = inputs.takes(&args);
    // A return type that cannot be returned is reported at that type.
    let result_span = match &sig.output {
        ReturnType::Type(_, ty) => ty.span(),
        ReturnType::Default => target.span(),
    };
    // Bound first, so that every token of the conversion is spanned at the
    // return type; see `private`. Unlike `ctx` and `args`, it is not
    // mixed-site, whose hygiene would break that span; no parameter can shadow
    // it, since the parameters are never bound as locals here.
    let result = Ident::new("result", result_span);
    let result_frostlang = private(result_span);
    let convert = quote_spanned!(result_span=> #result_frostlang::IntoNativeResult::into_result(#result, NAME));

    quote! {
        #(#cfgs)*
        #[doc = #doc]
        #vis fn #maker() -> #frostlang::Value {
            const NAME: &str = #name;
            const PARAMS: #frostlang::Params = #spec;
            #frostlang::Value::checked_native(NAME, PARAMS, |#ctx_pattern, #args| {
                let mut #args = #frostlang::Args::new(NAME, PARAMS, #args);
                let #result = #target(#ctx_arg #(#takes),*);
                #convert
            })
        }
    }
}
