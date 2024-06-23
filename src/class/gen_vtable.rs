//!
//! Copyright (C) Warsaw Revamped. Any unauthorized use, modification, or distribution of any portion of this file is prohibited. All rights reserved.
//!

use darling::FromAttributes;
use proc_macro2::Span;
use syn::Attribute;

use crate::class::extractor::AttributeExtractor;

#[derive(FromAttributes)]
#[darling(attributes(gen_vtable))]
pub struct GenVTable {
    #[darling(default)]
    pub no_unimpl: bool,
}

impl AttributeExtractor for GenVTable {
    type Output = Self;

    fn attr() -> &'static str {
        "gen_vtable"
    }

    fn parse_attr(attr: Attribute) -> syn::Result<Self::Output> {
        Self::from_attributes(&[attr]).map_err(|err| syn::Error::new(Span::call_site(), err))
    }
}
