use convert_case::{Case, Casing};
use itertools::Itertools;
use quote::{format_ident, quote};
use syn::{
    Attribute, Field, FieldMutability, File, ItemImpl, Meta, parse_quote, Token, Type, Visibility,
};
use syn::punctuated::Punctuated;

use crate::class::vtable::make_vtable_static;
use crate::parse::ItemClass;

/// Generates the base structure.
pub fn gen_struct(class: &ItemClass) -> File {
    let mut attrs = class.attrs.clone();

    let default_impl = intercept_default(class, &mut attrs);

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

            #default_impl
        }
        .into(),
    )
    .expect("failed to generate base struct")
}

/// Checks an attribute list for `repr(C)`
fn has_repr_c(attrs: &[Attribute]) -> bool {
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

/// Intercepts `#[derive(Default)]` and implements it ourselves
fn intercept_default(class: &ItemClass, attrs: &mut [Attribute]) -> Option<ItemImpl> {
    // see if there's a `derive` attribute
    let derive_attr = attrs
        .iter_mut()
        .find(|attr| attr.path().is_ident("derive"))?;

    // find `Default`
    let meta = derive_attr
        .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .expect("Failed to parse repr");

    // remove it... maybe someday we'll be able to do this
    let default_idx = meta
        .iter()
        .position(|meta| meta.path().is_ident("Default"))?;

    let mut new_meta = Punctuated::<Meta, Token![,]>::new();
    for (idx, meta) in meta.into_iter().enumerate() {
        if idx == default_idx {
            continue;
        }

        new_meta.push(meta);
    }

    // replace the old meta
    derive_attr.meta = parse_quote!(derive(#new_meta));

    // generate the implementation
    let arg_names = class
        .body
        .fields
        .iter()
        .filter_map(|field| field.ident.as_ref())
        .collect_vec();

    let ident = &class.ident;
    let vtable_static_ident = make_vtable_static(&class.ident);

    Some(
        syn::parse(
            quote! {
                impl Default for #ident {
                    fn default() -> Self {
                        Self {
                            vfptr: &#vtable_static_ident as *const _ as usize,
                            #(#arg_names: Default::default()),*
                        }
                    }
                }
            }
            .into(),
        )
        .expect("failed to generate default implementation"),
    )
}
