use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{Expr, File, FnArg, parse_quote, Pat, Path, TraitItemFn};

use crate::class::base_prefix;
use crate::parse::ItemClass;
use crate::util::{extract_ident, extract_implementor_generics, last_segment_mut};

/// Generates the virtuals trait for the type.
pub fn gen_trait(class: &ItemClass, no_unimpl: bool) -> File {
    let vis = &class.vis;
    let generics = &class.generics;
    let virtuals_ident = make_virtuals(&class.ident);

    // collect base trait identifiers
    let base_traits = collect_base_traits(class);

    // collect trait functions
    let trait_functions = collect_functions(class);

    // implement the macro
    let macro_impl = gen_unimpl_macro(class);

    // call the macro if needed
    let trait_impl = if !no_unimpl {
        let macro_ident = make_virtuals_macro_ident(&class.ident);
        let struct_ident = &class.ident;
        let struct_generic_args = class.generic_args();
        Some(quote!(#macro_ident!(#struct_ident, #struct_generic_args);))
    } else {
        None
    };

    let output = quote! {
        #vis trait #virtuals_ident #generics: #(#base_traits)+* {
            #(#trait_functions)*
        }

        #[allow(clippy::crate_in_macro_def)]
        #macro_impl
        #trait_impl
    };
    syn::parse(output.into()).expect("failed to generate trait")
}

/// Makes a class identifier refer to its virtuals trait.
pub fn make_virtuals(ident: &Ident) -> Ident {
    format_ident!("{}Virtuals", ident)
}

/// Make the VTable struct identifier.
pub fn make_virtuals_macro_ident(ident: &Ident) -> Ident {
    format_ident!("gen_{}_unimpl", ident.to_string().to_case(Case::Snake))
}

/// Collects a list of base trait identifiers.
fn collect_base_traits(class: &ItemClass) -> Vec<Path> {
    let prefix = base_prefix();

    class
        .bases
        .bases
        .iter()
        .cloned()
        .map(|(mut base, _)| {
            let segment = last_segment_mut(&mut base);
            segment.ident = make_virtuals(&segment.ident);
            parse_quote!(#prefix #base)
        })
        .collect()
}

/// Collects all functions as trait item functions.
fn collect_functions(class: &ItemClass) -> Vec<TraitItemFn> {
    class
        .body
        .virtuals
        .iter()
        .map(|virt| TraitItemFn {
            attrs: vec![],
            sig: virt.sig.clone(),
            default: None,
            semi_token: None,
        })
        .collect()
}

fn gen_unimpl_macro(class: &ItemClass) -> File {
    let struct_ident = &class.ident;
    let generic_args = class.generic_args().args;
    // collect all generic args into descriptors
    let def_generic_arg_idents = generic_args
        .iter()
        .enumerate()
        .map(|(idx, _)| format_ident!("def_generic_{idx}"))
        .collect_vec();
    let macro_ident = make_virtuals_macro_ident(struct_ident);
    let virtuals_ident = make_virtuals(struct_ident);

    let prefix = base_prefix();
    let impls = class
        .body
        .virtuals
        .iter()
        .map(|virt| {
            let mut sig = virt.sig.clone();

            // underscore all args
            for input in &mut sig.inputs {
                if let FnArg::Typed(arg) = input {
                    if let Pat::Ident(ident) = &mut *arg.pat {
                        ident.ident = format_ident!("_{}", ident.ident);
                    }
                }
            }
            quote!(
                #sig {
                    unimplemented!()
                }
            )
        })
        .collect_vec();

    // generate the base vtable
    let additional_impls: Vec<Expr> = class
        .bases
        .paths()
        .map(|base_path| {
            let base_ty = extract_ident(base_path);
            let macro_ident = make_virtuals_macro_ident(base_ty);

            // determine the position of each and extract it out of the parent definition
            let base_def_args = extract_implementor_generics(class, base_path);
            let expr = quote!(#macro_ident!($implementor_ty, <#(#base_def_args),*>));
            syn::parse(expr.into()).expect("failed to parse additional trait unimpl")
        })
        .collect();

    let output = quote! {
        #[macro_export]
        macro_rules! #macro_ident {
            // implementor_ty: The type of the implementor.
            // gen_x: The definition generic at position `x`.
            ($implementor_ty:ident, <#($#def_generic_arg_idents: tt),*>) => {
                #(#additional_impls;)*

                impl #prefix #virtuals_ident <#($#def_generic_arg_idents),*> for #prefix $implementor_ty {
                    #(#impls)*
                }
            }
        }
    };
    syn::parse(output.into()).expect("failed to generate unimpl macro")
}
