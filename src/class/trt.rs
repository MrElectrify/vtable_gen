use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{ItemTrait, Path, TraitItemFn};

use crate::parse::ItemClass;

/// Generates the virtuals trait for the type.
pub fn gen_trait(class: &ItemClass) -> ItemTrait {
    let vis = &class.vis;
    let generics = &class.generics;
    let virtuals_ident = make_virtuals(&class.ident);

    // collect base trait identifiers
    let base_traits = collect_base_traits(class);

    // collect trait functions
    let trait_functions = collect_functions(class);

    syn::parse(
        quote! {
            #vis trait #virtuals_ident #generics: #(#base_traits),* {
                #(#trait_functions)*
            }
        }
        .into(),
    )
    .expect("failed to generate trait")
}

/// Makes a class identifier refer to its virtuals trait.
pub fn make_virtuals(ident: &Ident) -> Ident {
    format_ident!("{}Virtuals", ident)
}

/// Collects a list of base trait identifiers.
fn collect_base_traits(class: &ItemClass) -> Vec<Path> {
    class
        .bases
        .bases
        .iter()
        .cloned()
        .map(|(mut base, _)| {
            let segment = base
                .segments
                .last_mut()
                .expect("expected base type segment");
            segment.ident = make_virtuals(&segment.ident);
            base
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
