use proc_macro2::{Ident, TokenStream};
use quote::ToTokens;
use syn::{
    AngleBracketedGenericArguments, Attribute, braced, Field, GenericParam, Generics, ItemImpl,
    LitInt, parenthesized, parse_quote, Path, Signature, token, Token, Visibility,
};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;

use crate::util::last_segment;

/// Base classes.
#[derive(Debug, Default, Clone)]
pub struct BaseClasses {
    pub colon_token: Option<Token![:]>,
    pub bases: Vec<(Path, Option<Token![,]>)>,
}

impl BaseClasses {
    /// Returns an iterator over all identifiers.
    pub fn idents(&self) -> impl Iterator<Item = &Ident> {
        self.paths().map(|path| &last_segment(path).ident)
    }

    /// Return the identifier of the base at `index`.
    pub fn ident(&self, index: usize) -> Option<&Ident> {
        self.path(index).map(|path| &last_segment(path).ident)
    }

    /// Returns true if there are no base classes.
    pub fn is_empty(&self) -> bool {
        self.bases.is_empty()
    }

    /// Return the path of the base at `index`.
    pub fn path(&self, index: usize) -> Option<&Path> {
        self.bases.get(index).map(|(path, _)| path)
    }

    /// Returns an iterator over all paths.
    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.bases.iter().map(|(path, _)| path)
    }
}

impl Parse for BaseClasses {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if !input.peek(Token![:]) {
            return Ok(BaseClasses::default());
        }

        let colon_token = input.parse()?;
        let mut bases = Vec::new();
        // keep parsing types until we hit the open brace
        loop {
            if input.is_empty() {
                break;
            }

            let ty = input.parse()?;
            let comma_token = input.parse()?;
            bases.push((ty, comma_token));

            if input.peek(token::Brace) {
                break;
            }
        }

        Ok(Self { colon_token, bases })
    }
}

/// The body of a class.
#[derive(Debug, Clone)]
pub struct ClassBody {
    braces: token::Brace,
    pub fields: Punctuated<Field, Token![,]>,
    pub virtuals: Punctuated<Virtual, Token![,]>,
}

impl Parse for ClassBody {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let braces = braced!(content in input);

        let mut fields = Punctuated::new();
        loop {
            if content.is_empty() || content.peek(Token![virtual]) {
                break;
            }

            // parse the field
            fields.push(content.call(Field::parse_named)?);
            if let Some(comma_token) = content.parse()? {
                fields.push_punct(comma_token);
            }
        }

        Ok(Self {
            braces,
            fields,
            virtuals: content.parse_terminated(Virtual::parse, Token![,])?,
        })
    }
}

/// A total class definition.
#[derive(Debug, Clone)]
pub struct ItemClass {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub struct_token: Token![struct],
    pub ident: Ident,
    pub generics: Generics,
    pub bases: BaseClasses,
    pub body: ClassBody,
}

impl ItemClass {
    /// Returns the generic arguments used.
    pub fn generic_args(&self) -> AngleBracketedGenericArguments {
        let mut args = Punctuated::new();
        for param in self.generics.params.iter() {
            let ident = match param {
                GenericParam::Type(ty) => &ty.ident,
                GenericParam::Lifetime(lt) => &lt.lifetime.ident,
                GenericParam::Const(ct) => &ct.ident,
            };

            args.push(parse_quote!(#ident))
        }

        AngleBracketedGenericArguments {
            colon2_token: None,
            lt_token: parse_quote!(<),
            args,
            gt_token: parse_quote!(>),
        }
    }
}

impl Parse for ItemClass {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            attrs: input.call(Attribute::parse_outer)?,
            vis: input.parse()?,
            struct_token: input.parse()?,
            ident: input.parse()?,
            generics: input.parse()?,
            bases: input.parse()?,
            body: input.parse()?,
        })
    }
}

/// A C++ Definition, containing a class and basic implementation.
#[derive(Debug, Clone)]
pub struct CppDef {
    pub class: ItemClass,
    pub new_impl: Option<ItemImpl>,
}

impl Parse for CppDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            class: input.parse()?,
            new_impl: if !input.is_empty() {
                Some(input.parse()?)
            } else {
                None
            },
        })
    }
}

/// Virtual functions.
#[derive(Debug, Clone)]
pub struct Virtual {
    pub virtual_token: Token![virtual],
    pub index: VirtualIndex,
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub sig: Signature,
}

impl Parse for Virtual {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            virtual_token: input.parse()?,
            index: input.parse()?,
            attrs: input.call(Attribute::parse_outer)?,
            vis: input.parse()?,
            sig: input.parse()?,
        })
    }
}

/// The index of a virtual method.
#[derive(Debug, Default, Clone)]
pub struct VirtualIndex {
    pub paren_token: Option<token::Paren>,
    pub idx: Option<LitInt>,
}

impl Parse for VirtualIndex {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if !input.peek(token::Paren) {
            return Ok(Self::default());
        }

        let content;
        Ok(Self {
            paren_token: Some(parenthesized!(content in input)),
            idx: Some(content.parse()?),
        })
    }
}

impl ToTokens for BaseClasses {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Some(colon_token) = self.colon_token {
            colon_token.to_tokens(tokens);
            for (ty, comma_token) in &self.bases {
                ty.to_tokens(tokens);
                comma_token.to_tokens(tokens);
            }
        }
    }
}

impl ToTokens for ClassBody {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.braces.surround(tokens, |tokens| {
            for pair in self.fields.pairs() {
                pair.value().to_tokens(tokens);
                pair.punct().to_tokens(tokens);
            }

            for pair in self.virtuals.pairs() {
                pair.value().to_tokens(tokens);
                pair.punct().to_tokens(tokens);
            }
        })
    }
}

impl ToTokens for CppDef {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.class.to_tokens(tokens);
        self.new_impl.to_tokens(tokens);
    }
}

impl ToTokens for ItemClass {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for attr in &self.attrs {
            attr.to_tokens(tokens);
        }

        self.vis.to_tokens(tokens);
        self.struct_token.to_tokens(tokens);
        self.ident.to_tokens(tokens);
        self.generics.to_tokens(tokens);
        self.body.to_tokens(tokens);
    }
}

impl ToTokens for Virtual {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.virtual_token.to_tokens(tokens);
        self.index.to_tokens(tokens);

        for attr in &self.attrs {
            attr.to_tokens(tokens);
        }

        self.vis.to_tokens(tokens);
        self.sig.to_tokens(tokens);
    }
}

impl ToTokens for VirtualIndex {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Some(paren_token) = self.paren_token.as_ref() {
            paren_token.surround(tokens, |tokens| self.idx.to_tokens(tokens))
        }
    }
}
