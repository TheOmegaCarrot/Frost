//! Converting what a Rust function or closure returns into what a Frost
//! function returns.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote_spanned};
use syn::spanned::Spanned;
use syn::{Ident, ReturnType};

use crate::private;

/// The local to bind a call's result to, and the expression converting it
/// through `IntoNativeResult` for the Frost function named by `name`.
///
/// A result that cannot be converted is reported at the return type in
/// `output`, or at `fallback` when there is none.
pub(crate) fn conversion(
    output: &ReturnType,
    fallback: Span,
    name: &impl ToTokens,
) -> (Ident, TokenStream) {
    let span = match output {
        ReturnType::Type(_, ty) => ty.span(),
        ReturnType::Default => fallback,
    };
    // Every token of the conversion is spanned at the return type, the local
    // too; see `private`. So the local is not mixed-site, whose hygiene would
    // break that span. Nothing can shadow it: the generated code binds no
    // parameter as a local, and it is bound in a scope of its own.
    let result = Ident::new("result", span);
    let frostlang = private(span);
    let convert = quote_spanned!(span=>
        #frostlang::IntoNativeResult::into_result(#result, #name)
    );
    (result, convert)
}
