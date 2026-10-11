//! `#[frost_bundle]` and `#[frost_mutable_bundle]`: a group of Frost functions
//! over shared state, from the functions of an impl block.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::ext::IdentExt;
use syn::parse::Parser;
use syn::{
    Attribute, FnArg, Ident, ImplItem, ImplItemFn, ItemImpl, LitStr, ReceiverKind, ReturnType,
};

use crate::errors::Errors;
use crate::inputs::Inputs;
use crate::{private, result, signature};

/// Which builder a bundle macro adds to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A `Bundle`, over state that never changes.
    Immutable,
    /// A `MutableBundle`, over state behind a lock.
    Mutable,
}

impl Kind {
    fn attribute(self) -> &'static str {
        match self {
            Kind::Immutable => "`#[frost_bundle]`",
            Kind::Mutable => "`#[frost_mutable_bundle]`",
        }
    }

    fn builder(self) -> TokenStream {
        let frostlang = private(Span::call_site());
        match self {
            Kind::Immutable => quote!(#frostlang::Bundle),
            Kind::Mutable => quote!(#frostlang::MutableBundle),
        }
    }
}

/// The attribute's arguments.
struct Group {
    /// What each function's key is qualified by in its name: `rng` names the
    /// key `int` `rng.int`.
    prefix: LitStr,
    /// The name of the generated function adding the group to a builder.
    name: Ident,
}

/// How a function takes the state.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Receiver {
    /// Not at all, as a context or as nothing.
    None,
    /// As `&self`.
    Shared,
    /// As `&mut self`.
    Mutable,
}

/// A function of the group.
struct Function {
    ident: Ident,
    /// Its name less the prefix, and its key in the bundle.
    key: String,
    cfgs: Vec<Attribute>,
    receiver: Receiver,
    inputs: Inputs,
    output: ReturnType,
    /// The span of its parameter list.
    params_span: Span,
}

/// The impl block as written, less its markers, and a second one with the
/// group's adder; or, for an invalid block, its errors, still with the block
/// so that its callers raise no further errors.
pub(crate) fn expand(kind: Kind, attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut block: ItemImpl = match syn::parse2(item) {
        Ok(block) => block,
        Err(error) => return error.into_compile_error(),
    };
    let mut errors = Errors::default();
    let group = errors.keep(group(kind, attr));
    check_block(kind, &block, &mut errors);
    let mut functions = Vec::new();
    for item in &mut block.items {
        match item {
            ImplItem::Fn(function) => functions.extend(read_function(kind, function, &mut errors)),
            other => errors.push(syn::Error::new_spanned(
                other,
                format!(
                    "{} takes an impl block of functions only: move this to an impl block of \
                     its own",
                    kind.attribute()
                ),
            )),
        }
    }
    match (errors.finish(), group) {
        (Ok(()), Some(group)) => {
            let adder = adder(kind, &block, &group, &functions);
            quote!(#block #adder)
        }
        (result, _) => {
            let error = result.err().map(syn::Error::into_compile_error);
            quote!(#error #block)
        }
    }
}

/// The group's prefix and adder name, from the attribute's arguments.
fn group(kind: Kind, attr: TokenStream) -> syn::Result<Group> {
    let (mut prefix, mut name) = (None, None);
    let parser = syn::meta::parser(|meta| {
        let slot_is_free = if meta.path.is_ident("prefix") {
            prefix.is_none()
        } else if meta.path.is_ident("group") {
            name.is_none()
        } else {
            return Err(meta.error(format!(
                "unknown argument: {} takes `prefix` and `group`",
                kind.attribute()
            )));
        };
        if !slot_is_free {
            return Err(meta.error("this argument is given twice"));
        }
        let value = meta.value()?;
        if meta.path.is_ident("prefix") {
            prefix = Some(value.parse()?);
        } else {
            name = Some(value.parse()?);
        }
        Ok(())
    });
    parser.parse2(attr)?;
    let prefix = prefix.ok_or_else(|| {
        syn::Error::new(
            Span::call_site(),
            format!(
                "{} needs the prefix qualifying its functions' names: `prefix = \"...\"`",
                kind.attribute()
            ),
        )
    })?;
    let name = name.unwrap_or_else(|| Ident::new("frost_functions", Span::call_site()));
    Ok(Group { prefix, name })
}

/// Records an error for each way `block` is not a plain inherent impl block.
fn check_block(kind: Kind, block: &ItemImpl, errors: &mut Errors) {
    let not = |thing: &str| format!("{} takes a plain impl block, not {thing}", kind.attribute());
    errors.keep(block.modifiers.require_empty());
    if let Some(unsafety) = &block.unsafety {
        errors.push(syn::Error::new_spanned(unsafety, not("an `unsafe` one")));
    }
    if let Some((path, for_token)) = &block.trait_ {
        errors.push(syn::Error::new_spanned(
            quote!(#path #for_token),
            not("a trait impl"),
        ));
    }
    let generic = not("a generic one: the macro must name each parameter's type");
    if !block.generics.params.is_empty() {
        errors.push(syn::Error::new_spanned(&block.generics, &generic));
    } else if let Some(where_clause) = &block.generics.where_clause {
        errors.push(syn::Error::new_spanned(where_clause, &generic));
    }
}

/// Reads `function`, stripping its parameters' markers.
fn read_function(kind: Kind, function: &mut ImplItemFn, errors: &mut Errors) -> Option<Function> {
    let lead = format!("{} needs a plain function", kind.attribute());
    signature::check(&function.modifiers, &function.sig, &lead, errors);
    let receiver = match function.sig.receiver() {
        None => Some(Receiver::None),
        Some(receiver) => match (&receiver.kind, kind) {
            (ReceiverKind::Reference(_, _, None), _) => Some(Receiver::Shared),
            (ReceiverKind::Reference(_, _, Some(_)), Kind::Mutable) => Some(Receiver::Mutable),
            (ReceiverKind::Reference(_, _, Some(_)), Kind::Immutable) => {
                errors.push(syn::Error::new_spanned(
                    receiver,
                    "`#[frost_bundle]` is over state that never changes, so its functions \
                     take `&self`; for state that changes, see `#[frost_mutable_bundle]`",
                ));
                None
            }
            _ => {
                let takes = match kind {
                    Kind::Immutable => "`&self`",
                    Kind::Mutable => "`&self` or `&mut self`",
                };
                errors.push(syn::Error::new_spanned(
                    receiver,
                    format!(
                        "{} lends its functions the state: take it as {takes}",
                        kind.attribute()
                    ),
                ));
                None
            }
        },
    };
    let mut inputs = Inputs::default();
    for input in &mut function.sig.inputs {
        if let FnArg::Typed(typed) = input {
            inputs.read(&mut typed.attrs, &typed.pat, Some(&typed.ty), errors);
        }
    }
    let ident = function.sig.ident.clone();
    Some(Function {
        key: ident.unraw().to_string(),
        ident,
        cfgs: function
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("cfg"))
            .cloned()
            .collect(),
        receiver: receiver?,
        inputs,
        output: function.sig.output.clone(),
        params_span: function.sig.paren_token.span.join(),
    })
}

/// A second impl block on the type, with the adder: the group's functions
/// added to a builder, each by a `let` of its own so that it can carry its
/// function's `#[cfg]`s.
fn adder(kind: Kind, block: &ItemImpl, group: &Group, functions: &[Function]) -> TokenStream {
    let self_ty = &block.self_ty;
    let builder = kind.builder();
    let bundle = Ident::new("bundle", Span::mixed_site());
    let prefix = group.prefix.value();
    let names: Vec<String> = functions
        .iter()
        .map(|function| format!("{prefix}.{}", function.key))
        .collect();
    let doc = format!(
        "Adds the Frost functions {} to `bundle`.",
        names
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let prefix_check = usable_name(
        &prefix,
        group.prefix.span(),
        &format!("a bundle's prefix must be a Frost identifier, not a keyword: `{prefix}`"),
    );
    let adds = functions.iter().zip(&names).map(|(function, name)| {
        let key_check = usable_name(
            &function.key,
            function.ident.span(),
            &format!(
                "a bundle's keys must be Frost identifiers, not keywords: rename `{}`",
                function.key
            ),
        );
        let cfgs = &function.cfgs;
        let call = function.call(kind, name);
        quote! {
            #key_check
            #(#cfgs)*
            let #bundle = #bundle.function(#name, #call);
        }
    });
    let group_name = &group.name;
    // The adder exists exactly when the block does.
    let cfgs = block
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"));
    quote! {
        #(#cfgs)*
        impl #self_ty {
            #[doc = #doc]
            fn #group_name(#bundle: #builder<Self>) -> #builder<Self> {
                #prefix_check
                #(#adds)*
                #bundle
            }
        }
    }
}

/// A compile-time check that `name` is usable as a Frost identifier, failing
/// with `message` at `span`.
fn usable_name(name: &str, span: Span, message: &str) -> TokenStream {
    let frostlang = private(span);
    quote_spanned!(span=>
        const _: () = ::core::assert!(
            #frostlang::is_identifier_like_and_not_keyword(#name),
            #message,
        );
    )
}

impl Function {
    /// The rest of the arguments to the builder's `function` after the name:
    /// the spec, and the closure taking the arguments and calling this
    /// function.
    fn call(&self, kind: Kind, name: &str) -> TokenStream {
        // Mixed-site, so that no parameter name can shadow them.
        let [ctx, args, state] =
            ["ctx", "args", "state"].map(|n| Ident::new(n, Span::mixed_site()));
        let spec = self.inputs.spec(self.params_span);
        let arg_locals: Vec<Ident> = (0..self.inputs.args.len())
            .map(|i| format_ident!("arg{i}", span = Span::mixed_site()))
            .collect();
        let takes = self.inputs.takes(&args);
        let (result, convert) = result::conversion(&self.output, self.ident.span(), &name);
        let method = &self.ident;
        let uses_ctx = self.receiver != Receiver::None || self.inputs.ctx.is_some();
        let ctx_pattern = if uses_ctx { quote!(#ctx) } else { quote!(_) };

        // Called as `Self::method`, so that no method of the context or the
        // guard can stand in for one of the same name.
        let call = match (self.receiver, kind) {
            (Receiver::None, _) => {
                let (pass_ctx, ctx_arg) = self.inputs.pass_ctx(quote!(#ctx));
                quote! {
                    #pass_ctx
                    let #result = Self::#method(#ctx_arg #(#arg_locals),*);
                }
            }
            (_, Kind::Immutable) => {
                // The state first: it borrows the bundle, not `ctx`, which
                // `native` then borrows.
                let (pass_ctx, ctx_arg) = self.inputs.pass_ctx(quote!(#ctx.native()));
                quote! {
                    let #state = #ctx.state();
                    #pass_ctx
                    let #result = Self::#method(#state, #ctx_arg #(#arg_locals),*);
                }
            }
            (receiver, Kind::Mutable) => {
                let (binding, borrow) = if receiver == Receiver::Mutable {
                    (quote!(mut #state), quote!(&mut *#state))
                } else {
                    (quote!(#state), quote!(&*#state))
                };
                let (pass_ctx, ctx_arg) = self.inputs.pass_ctx(quote!(#state.native()));
                quote! {
                    let #binding = #ctx.lock()?;
                    #pass_ctx
                    let #result = Self::#method(#borrow, #ctx_arg #(#arg_locals),*);
                }
            }
        };

        quote! {
            const { #spec },
            |#ctx_pattern, mut #args| {
                // Taken before the state is, so that it is held no longer than
                // the call.
                #(let #arg_locals = #takes;)*
                #call
                #convert
            }
        }
    }
}
