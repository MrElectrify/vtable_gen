use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{File, parse_quote, Path, TraitItemFn};

use crate::class::base_prefix;
use crate::parse::ItemClass;
use crate::util::last_segment_mut;

/// Generates the virtuals trait for the type.
pub fn gen_trait(class: &ItemClass) -> File {
    let vis = &class.vis;
    let generics = &class.generics;
    let virtuals_ident = make_virtuals(&class.ident);

    // collect base trait identifiers
    let base_traits = collect_base_traits(class);

    // collect trait functions
    let trait_functions = collect_functions(class);

    let output = quote! {
        #vis trait #virtuals_ident #generics: #(#base_traits)+* {
            #(#trait_functions)*
        }
    };
    syn::parse(output.into()).expect("failed to generate trait")
}

/// Makes a class identifier refer to its virtuals trait.
pub fn make_virtuals(ident: &Ident) -> Ident {
    format_ident!("{}Virtuals", ident)
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
