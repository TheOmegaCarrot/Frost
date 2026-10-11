//! The macros behind `frostlang`'s Frost function bindings. Use them through
//! `frostlang`, which re-exports and documents them.

mod bundle;
mod closure;
mod errors;
mod frost_fn;
mod inputs;
mod result;
mod signature;

use proc_macro::TokenStream;

// Documented at its re-export in `frostlang`.
#[proc_macro_attribute]
pub fn frost_fn(attr: TokenStream, item: TokenStream) -> TokenStream {
    frost_fn::expand(attr.into(), item.into()).into()
}

// Documented at its re-export in `frostlang`.
#[proc_macro]
pub fn frost_closure(input: TokenStream) -> TokenStream {
    closure::expand(input.into()).into()
}

// Documented at its re-export in `frostlang`.
#[proc_macro_attribute]
pub fn frost_bundle(attr: TokenStream, item: TokenStream) -> TokenStream {
    bundle::expand(bundle::Kind::Immutable, attr.into(), item.into()).into()
}

// Documented at its re-export in `frostlang`.
#[proc_macro_attribute]
pub fn frost_mutable_bundle(attr: TokenStream, item: TokenStream) -> TokenStream {
    bundle::expand(bundle::Kind::Mutable, attr.into(), item.into()).into()
}

/// The path through which generated code reaches `frostlang`: never its public
/// paths, so those may move without breaking the macros.
///
/// Spanned at `span`, since rustc reports an error in an expression at the
/// expression's whole span only if all its tokens are spanned alike.
fn private(span: proc_macro2::Span) -> proc_macro2::TokenStream {
    quote::quote_spanned!(span=> ::frostlang::__private)
}
