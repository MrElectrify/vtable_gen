use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{Attribute, bracketed, Path, Token};
use syn::parse::{Parse, ParseStream};

use crate::class::extractor::AttributeExtractor;

/// A generic base class replacement.
#[derive(Clone)]
pub struct GenericBase {
    pub base: Ident,
    pub eq: Token![=],
    pub repl: Path,
}

impl AttributeExtractor for GenericBase {
    type Output = Vec<Vec<GenericBase>>;

    fn attr() -> &'static str {
        "impl_generic_base"
    }

    fn parse_attr(attr: Attribute) -> syn::Result<Self::Output> {
        Ok(attr
            .parse_args_with(|args: ParseStream| {
                args.parse_terminated(
                    |base_collection| {
                        let contents;
                        bracketed!(contents in base_collection);

                        contents.parse_terminated(Self::parse, Token![,])
                    },
                    Token![,],
                )
            })?
            .into_iter()
            .map(|p| p.into_iter().collect())
            .collect())
    }
}

impl Parse for GenericBase {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            base: input.parse()?,
            eq: input.parse()?,
            repl: input.parse()?,
        })
    }
}

impl ToTokens for GenericBase {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.base.to_tokens(tokens);
        self.eq.to_tokens(tokens);
        self.repl.to_tokens(tokens);
    }
}
