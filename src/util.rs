use std::collections::HashMap;
use std::iter;

use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{parse_quote, Path, PathArguments, PathSegment};
use syn::punctuated::Punctuated;

use crate::parse::ItemClass;

/// Collects secondary base classes.
pub fn collect_secondary_bases<'a>(
    class: &'a ItemClass,
    additional_bases: &'a HashMap<Path, Vec<Path>>,
) -> Vec<&'a Path> {
    class
        .bases
        .paths()
        .flat_map(|path| {
            iter::once(path)
                .chain(
                    additional_bases
                        .get(path)
                        .iter()
                        .flat_map(|paths| paths.iter()),
                )
                .collect_vec()
        })
        .skip(1)
        .collect()
}

/// Extracts an identifier out of the end of a path.
pub fn extract_ident(path: &Path) -> &Ident {
    &last_segment(path).ident
}

/// Extracts generics from an implementor.
pub fn extract_implementor_generics(class: &ItemClass, base_path: &Path) -> Vec<TokenStream> {
    let class_generics = class.generic_args();

    // extract the angle bracketed_arguments
    let def_generics = match &base_path
        .segments
        .last()
        .expect("expected path segment")
        .arguments
    {
        PathArguments::AngleBracketed(def_generics) => def_generics.clone(),
        _ => parse_quote!(<>),
    };

    // determine the position of each and extract it out of the parent definition
    def_generics
        .args
        .iter()
        .map(|base_arg| {
            class_generics
                .args
                .iter()
                .position(|class_arg| class_arg == base_arg)
                .map(|idx| {
                    let ident = format_ident!("def_generic_{idx}");
                    quote! { $#ident }
                })
                .unwrap_or_else(|| base_arg.to_token_stream())
        })
        .collect()
}

pub fn last_segment(path: &Path) -> &PathSegment {
    path.segments.last().expect("expected path segments")
}

pub fn last_segment_mut(path: &mut Path) -> &mut PathSegment {
    path.segments.last_mut().expect("expected path segments")
}

/// Removes a field from a punctuation.
pub fn remove_punctuated<T: Clone, P: Clone, F: FnMut(&T) -> bool>(
    punct: &Punctuated<T, P>,
    mut pred: F,
) -> Punctuated<T, P> {
    let mut new_punct = Punctuated::new();
    for item in punct {
        if pred(item) {
            new_punct.push_value(item.clone());
        }
    }
    new_punct
}
