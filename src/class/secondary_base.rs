use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{Attribute, bracketed, Path, token, Token};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;

use crate::class::extractor::AttributeExtractor;

/// A secondary base class.
pub struct SecondaryBase {
    pub target: Path,
    pub eq: Token![=],
    pub bracket: token::Bracket,
    pub bases: Punctuated<Path, Token![,]>,
}

impl AttributeExtractor for SecondaryBase {
    type Output = HashMap<Path, Vec<Path>>;

    fn attr() -> &'static str {
        "gen_base"
    }

    fn parse_attr(attr: Attribute) -> syn::Result<Self::Output> {
        Ok(attr
            .parse_args_with(Punctuated::<Self, Token![,]>::parse_terminated)?
            .iter()
            .map(|expr| (expr.target.clone(), expr.bases.iter().cloned().collect()))
            .collect())
    }
}

impl Parse for SecondaryBase {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let target = input.parse()?;
        let eq = input.parse()?;

        let contents;
        let bracket = bracketed!(contents in input);

        Ok(Self {
            target,
            eq,
            bracket,
            bases: contents.parse_terminated(Path::parse, Token![,])?,
        })
    }
}

impl ToTokens for SecondaryBase {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.target.to_tokens(tokens);
        self.eq.to_tokens(tokens);
        self.bracket.surround(tokens, |tokens| {
            for base in &self.bases {
                base.to_tokens(tokens);
            }
        })
    }
}
