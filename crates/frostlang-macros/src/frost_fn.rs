//! `#[frost_fn]`: a Frost function calling a Rust function.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::parse::Parser;
use syn::{FnArg, Ident, ItemFn, LitStr, Signature};

use crate::errors::Errors;
use crate::inputs::Inputs;
use crate::{private, result, signature, usable_name};

/// The function as written, less its markers, and its maker; or, for an
/// invalid declaration, its errors, still with the function so that its
/// callers raise no further errors.
pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut function: ItemFn = match syn::parse2(item) {
        Ok(function) => function,
        Err(error) => return error.into_compile_error(),
    };
    let mut errors = Errors::default();
    let name = errors.keep(base_name(attr));
    signature::check(
        &function.modifiers,
        &function.sig,
        "`#[frost_fn]` takes a plain function",
        &mut errors,
    );
    let inputs = read_inputs(&mut function.sig, &mut errors);
    match (errors.finish(), name) {
        (Ok(()), Some(name)) => {
            let maker = maker(&function, name.as_ref(), &inputs);
            quote!(#function #maker)
        }
        (result, _) => {
            let error = result.err().map(syn::Error::into_compile_error);
            quote!(#error #function)
        }
    }
}

/// The base name the attribute's `name = "..."` gives, if any.
fn base_name(attr: TokenStream) -> syn::Result<Option<LitStr>> {
    let mut name: Option<LitStr> = None;
    let parser = syn::meta::parser(|meta| {
        if !meta.path.is_ident("name") {
            return Err(meta.error("unknown argument: `#[frost_fn]` takes only `name`"));
        }
        if name.is_some() {
            return Err(meta.error("`name` is given twice"));
        }
        let given: LitStr = meta.value()?.parse()?;
        if given.value().contains('.') {
            return Err(syn::Error::new_spanned(
                given,
                "a base name is one segment, with no `.`: \
                 where the function is placed gives its path",
            ));
        }
        name = Some(given);
        Ok(())
    });
    parser.parse2(attr)?;
    Ok(name)
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

/// `<function>_frost`, returning the definition of the Frost function, whose
/// base name is `name`, or else the function's own.
fn maker(function: &ItemFn, name: Option<&LitStr>, inputs: &Inputs) -> TokenStream {
    let frostlang = private(Span::call_site());
    let (vis, sig) = (&function.vis, &function.sig);
    let target = &sig.ident;
    let maker = format_ident!("{}_frost", target.unraw());
    let doc = format!("The definition of the Frost function made from [`{target}`].");
    // The maker exists exactly when the function does.
    let cfgs = function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"));

    let (base_name, base_span) = match name {
        Some(name) => (name.value(), name.span()),
        None => (target.unraw().to_string(), target.span()),
    };
    let check = usable_name(
        &base_name,
        base_span,
        &format!(
            "a base name must be a Frost identifier, not a keyword: \
             give another with `name = \"...\"`, not `{base_name}`"
        ),
    );

    // Mixed-site, so that no parameter name can shadow them.
    let [name, ctx, args] = ["name", "ctx", "args"].map(|n| Ident::new(n, Span::mixed_site()));
    // The context is always bound, for its name; mutably only to be passed on.
    let ctx_pattern = match inputs.ctx {
        Some(_) => quote!(mut #ctx),
        None => quote!(#ctx),
    };
    let (pass_ctx, ctx_arg) = inputs.pass_ctx(quote!(&mut #ctx));
    let spec = inputs.spec(sig.paren_token.span.join());
    let takes = inputs.takes(&args);
    let (result, convert) = result::conversion(&sig.output, target.span(), &name);

    quote! {
        #(#cfgs)*
        #[doc = #doc]
        #vis const fn #maker() -> #frostlang::NativeDef {
            #check
            const PARAMS: #frostlang::Params = #spec;
            #frostlang::native_def(#base_name, |#name| {
                #frostlang::Value::checked_native(#name, PARAMS, |#ctx_pattern, #args| {
                    let #name = #ctx.name();
                    let mut #args = #frostlang::Args::new(#name, PARAMS, #args);
                    #pass_ctx
                    let #result = #target(#ctx_arg #(#takes),*);
                    #convert
                })
            })
        }
    }
}
