use convert_case::{Case, Casing};
use quote::{format_ident, quote};
use syn::{
    Attribute, Field, FieldMutability, ItemStruct, Meta, parse_quote, Token, Type, Visibility,
};
use syn::punctuated::Punctuated;

use crate::parse::ItemClass;

/// Generates the base structure.
pub fn gen_struct(class: &ItemClass) -> ItemStruct {
    let mut attrs = class.attrs.clone();
    let vis = &class.vis;
    let ident = &class.ident;
    let generics = &class.generics;

    // add the bases to the fields list
    let mut fields = class.body.fields.clone();
    for (base, _) in &class.bases.bases {
        let base_ident_camel_case = base
            .path
            .segments
            .last()
            .expect("expected base type")
            .ident
            .to_string()
            .to_case(Case::Snake);

        fields.push(Field {
            attrs: vec![],
            vis: Visibility::Inherited,
            mutability: FieldMutability::None,
            ident: Some(format_ident!("base_{base_ident_camel_case}")),
            colon_token: None,
            ty: Type::Path(base.clone()),
        })
    }

    // add the vtable if there aren't any bases and there are virtuals.
    if class.bases.bases.is_empty() && !class.body.virtuals.is_empty() {
        // push the VTable member
        fields.push(Field {
            attrs: vec![],
            vis: Visibility::Inherited,
            mutability: FieldMutability::None,
            ident: Some(format_ident!("vfptr")),
            colon_token: None,
            ty: parse_quote!(usize),
        })
    }

    // non-virtual bases are not supported because we don't have a way
    // of knowing if the first base class is virtual or not otherwise,
    // and we wouldn't know if we need to generate a vtable
    if class.body.virtuals.is_empty() && class.bases.bases.is_empty() {
        panic!("non-virtual base-classes are not supported")
    }

    // if there's not `#[repr(C)]`, add it
    if !has_repr_c(&attrs) {
        attrs.push(parse_quote!(#[repr(C)]));
    }

    syn::parse(
        quote! {
            #(#attrs)*
            #vis struct #ident #generics {
                #fields
            }
        }
        .into(),
    )
    .expect("failed to generate base struct")
}

/// Checks an attribute list for `repr(C)`
pub fn has_repr_c(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        if !attr.path().is_ident("repr") {
            return false;
        }

        // parse metas
        let nested = attr
            .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .expect("Failed to parse repr");

        nested.iter().any(|meta| meta.path().is_ident("C"))
    })
}
