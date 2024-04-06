#![doc = include_str!("../README.md")]

use std::collections::HashSet;

use auto_enums::auto_enum;
use darling::{FromAttributes, FromMeta};
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, Attribute, Block, ConstParam, Expr,
    FnArg, GenericParam, Generics, Ident, ItemFn, Pat, PatIdent, PatType, Result, Signature, Token,
    Type,
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

fn remove_attr(arg: FnArg) -> FnArg {
    match arg {
        FnArg::Typed(mut typed) => {
            typed.attrs.clear();
            FnArg::Typed(typed)
        }
        FnArg::Receiver(receiver) => FnArg::Receiver(receiver),
    }
}

fn contains_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("maybe_const"))
}

#[auto_enum]
fn inner(_attr: TokenStream, item: ItemFn) -> Result<TokenStream> {
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
                if !contains_attr(attrs) {
                    return None;
                }
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

    let old_fn_name = format_ident!("{ident}_orig");

    let orig_const_args: Vec<_> = generics
        .const_params()
        .map(|param| param.ident.clone())
        .collect();

    let fns = targets
        .iter()
        .cloned()
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
            let new_fn_ident = if set.is_empty() {
                old_fn_name.clone()
            } else {
                Ident::new(&new_fn_name, ident.span())
            };

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
                    .map(|(_idx, input)| input)
                    .map(remove_attr)
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
            ItemFn {
                sig: new_sig,
                attrs: new_attrs,
                ..item
            }
        })
        .collect::<Vec<_>>();

    // Generate the dispatch function
    let all_target_names = targets
        .iter()
        .map(|target| target.arg_name.clone())
        .collect::<Vec<_>>();

    let mut branches = targets
        .iter()
        .cloned()
        .enumerate()
        .powerset()
        .flat_map(|set| {
            let new_fn_name = [ident.to_string()]
                .into_iter()
                .chain(set.iter().map(|(_, t)| {
                    t.attr
                        .dispatch
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or(t.arg_name.to_string())
                }))
                .join("_");
            let new_fn_ident = if set.is_empty() {
                old_fn_name.clone()
            } else {
                Ident::new(&new_fn_name, ident.span())
            };

            let remain_args = {
                let args_to_remove: HashSet<_> = set.iter().map(|(_, t)| t.idx).collect();
                inputs
                    .iter()
                    .cloned()
                    .enumerate()
                    .filter(|(idx, _)| !args_to_remove.contains(idx))
                    .map(|(_idx, input)| input)
                    .map(|input| match input {
                        FnArg::Receiver(_reciver) => quote! { self },
                        FnArg::Typed(typed) => match *typed.pat {
                            Pat::Ident(pat_ident) => {
                                let name = pat_ident.ident;
                                quote! { #name }
                            }
                            _ => panic!("Only support simple pattern"),
                        },
                    })
                    .collect::<Vec<_>>()
            };

            #[auto_enum(Iterator)]
            let const_sets = if set.is_empty() {
                std::iter::once(vec![])
            } else {
                Itertools::multi_cartesian_product(set.iter().map(|(idx, target)| {
                    itertools::izip!(std::iter::repeat(idx), target.attr.consts.inner.iter(),)
                }))
            };

            const_sets
                .map(|const_set| {
                    let mut match_args = all_target_names
                        .iter()
                        .map(|target_name| quote! { #target_name })
                        .collect::<Vec<_>>();
                    let mut added_const_args = Vec::with_capacity(const_set.len());
                    for (idx_in_target, r#const) in const_set {
                        match_args[*idx_in_target] = quote! { #r#const };
                        added_const_args.push(quote! { #r#const });
                    }
                    let const_args = orig_const_args
                        .iter()
                        .map(|ident| quote! { #ident })
                        .chain(added_const_args.into_iter());
                    if remain_args.is_empty() {
                        quote! {
                            (#(#match_args),*) => {
                                #new_fn_ident::<#(#const_args),*>()
                            }
                        }
                    } else {
                        quote! {
                            (#(#match_args),*) => {
                                #new_fn_ident::<#(#const_args),*>(#(#remain_args),*,)
                            }
                        }
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    branches.reverse();

    let dispatch_fn = {
        let body: Block = parse_quote! {
            {
                match (#(#all_target_names),*) {
                    #(#branches),*
                }
            }
        };
        let new_inputs = sig
            .inputs
            .iter()
            .cloned()
            .map(remove_attr)
            .collect::<Punctuated<_, Token![,]>>();
        let new_sig = Signature {
            inputs: new_inputs,
            ..sig
        };
        let mut new_attrs = item2.attrs.clone();
        new_attrs.push(parse_quote! { #[inline(always)] });
        ItemFn {
            sig: new_sig,
            block: Box::new(body),
            attrs: new_attrs,
            ..item2
        }
    };

    Ok(quote! {
        #dispatch_fn
        #(#fns)*
    })
}
