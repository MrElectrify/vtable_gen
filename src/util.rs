use proc_macro2::Ident;
use syn::{Path, PathSegment};

/// Extracts an identifier out of the end of a path.
pub fn extract_ident(path: &Path) -> &Ident {
    &last_segment(path).ident
}

pub fn last_segment(path: &Path) -> &PathSegment {
    path.segments.last().expect("expected path segments")
}
