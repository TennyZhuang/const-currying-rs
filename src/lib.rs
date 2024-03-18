use std::collections::HashSet;

use darling::{FromAttributes, FromMeta};
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, Attribute, ConstParam, Expr, FnArg,
    GenericParam, Generics, Ident, ItemFn, Lit, LitInt, Pat, PatIdent, PatType, Result, Signature,
    Token, Type,
};

#[proc_macro_attribute]
pub fn const_currying(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    match inner(attr.into(), input) {
        Ok(output) => output.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[derive(Debug, Clone, darling::FromAttributes)]
#[darling(attributes(maybe_const))]
struct FieldAttr {
    #[darling(default)]
    dispatch: Option<Ident>,
    #[darling(default)]
    consts: ConstsArray,
}

#[derive(Debug, Clone, Default)]
struct ConstsArray {
    inner: Punctuated<Expr, Token![,]>,
}

impl FromMeta for ConstsArray {
    fn from_expr(expr: &Expr) -> darling::Result<Self> {
        if let Expr::Array(array) = expr {
            Ok(Self {
                inner: array.elems.clone(),
            })
        } else {
            Err(darling::Error::unexpected_expr_type(expr))
        }
    }
}

#[derive(Clone, Debug)]
struct GenTarget {
    attr: FieldAttr,
    idx: usize,
    arg_name: Ident,
    input: PatType,
    ty: Type,
}

fn inner(attr: TokenStream, item: ItemFn) -> Result<TokenStream> {
    let item2 = item.clone();
    let ItemFn { sig, .. } = item;

    let Signature {
        ident,
        inputs,
        generics,
        ..
    } = &sig;

    let targets = inputs
        .iter()
        .enumerate()
        .filter_map(|(idx, input)| match input {
            FnArg::Receiver(..) => None,
            FnArg::Typed(typed) => {
                let PatType { attrs, ty, pat, .. } = typed;
                let Pat::Ident(PatIdent {
                    ident: arg_name, ..
                }) = &**pat
                else {
                    return None;
                };
                let attr = FieldAttr::from_attributes(attrs).ok()?;
                Some(GenTarget {
                    attr,
                    idx,
                    arg_name: arg_name.clone(),
                    input: typed.clone(),
                    ty: *ty.clone(),
                })
            }
        })
        .collect::<Vec<_>>();

    let fns = targets
        .into_iter()
        .powerset()
        .zip(std::iter::from_fn(|| {
            let item = item2.clone();
            Some(item)
        }))
        .map(|(set, item)| {
            let ItemFn { sig, .. } = item.clone();
            let Signature {
                ident,
                inputs,
                generics,
                ..
            } = &sig;
            let new_fn_name = [ident.to_string()]
                .into_iter()
                .chain(set.iter().map(|t| {
                    t.attr
                        .dispatch
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or(t.arg_name.to_string())
                }))
                .join("_");
            let new_fn_ident = Ident::new(&new_fn_name, ident.span());

            let added_generic_params = set
                .iter()
                .map(|t: &GenTarget| {
                    let GenTarget {
                        attr: _,
                        idx: _,
                        arg_name,
                        input,
                        ty,
                    } = t;
                    ConstParam {
                        attrs: vec![],
                        const_token: Token![const](arg_name.span()),
                        ident: arg_name.clone(),
                        colon_token: input.colon_token,
                        ty: ty.clone(),
                        default: None,
                        eq_token: None,
                    }
                })
                .map(GenericParam::Const);

            let mut old_generics_pararms = generics.params.clone();
            for new_param in added_generic_params {
                old_generics_pararms.push(new_param);
            }
            let new_generics = Generics {
                params: old_generics_pararms,
                ..generics.clone()
            };
            let new_inputs = {
                let args_to_remove: HashSet<_> = set.iter().map(|t| t.idx).collect();
                inputs
                    .iter()
                    .cloned()
                    .enumerate()
                    .filter(|(idx, _)| !args_to_remove.contains(idx))
                    .map(|(_idx, input)| {
                        let FnArg::Typed(mut typed) = input else {
                            return input;
                        };
                        typed
                            .attrs
                            .retain(|attr| !attr.path().is_ident("maybe_const"));
                        FnArg::Typed(typed)
                    })
                    .collect::<Punctuated<_, Token![,]>>()
            };
            let sig = sig.clone();
            let new_sig = Signature {
                ident: new_fn_ident,
                inputs: new_inputs,
                generics: new_generics,
                ..sig
            };
            let item = item.clone();
            let mut new_attrs = item.attrs.clone();
            let new_attr: Attribute = parse_quote!(#[allow(warnings)]);
            new_attrs.push(new_attr);
            let new_fn = ItemFn {
                sig: new_sig,
                attrs: new_attrs,
                ..item
            };
            new_fn
        })
        .collect::<Vec<_>>();

    Ok(quote! {
        // #item2
        #(#fns)*
    })
}
