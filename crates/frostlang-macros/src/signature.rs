//! Checking that a Rust function is plain enough to call as a Frost function.

use syn::{FnModifiers, Safety, Signature};

use crate::errors::Errors;

/// Records an error for each way a function, with `modifiers` and `sig`, is
/// not a plain function. Each error begins with `lead`, such as "`#[frost_fn]`
/// takes a plain function", and goes on to say what it is instead.
pub(crate) fn check(modifiers: &FnModifiers, sig: &Signature, lead: &str, errors: &mut Errors) {
    let not = |thing: &str| format!("{lead}, not {thing}");
    errors.keep(modifiers.require_empty());
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
