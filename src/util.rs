use std::collections::HashMap;
use std::iter;

use itertools::Itertools;
use proc_macro2::Ident;
use syn::{Path, PathSegment};

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

pub fn last_segment(path: &Path) -> &PathSegment {
    path.segments.last().expect("expected path segments")
}
