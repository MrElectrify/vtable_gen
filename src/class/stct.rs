use std::collections::HashMap;

use itertools::Itertools;
use quote::quote;
use syn::{Attribute, FieldValue, File, Meta, parse_quote, Path, Token};
use syn::punctuated::Punctuated;

use crate::class::{imp, make_base_name};
use crate::class::vtable::make_vtable_ident;
use crate::parse::ItemClass;

/// Generates the base structure.
pub fn gen_struct(class: &ItemClass, additional_bases: &HashMap<Path, Vec<Path>>) -> File {
    let mut attrs = class.attrs.clone();

    let default_impl = intercept_default(class, &mut attrs, additional_bases);

    let vis = &class.vis;
    let ident = &class.ident;
    let generics = &class.generics;

    // add the bases to the fields list
    let mut fields = class.body.fields.clone();
    for (idx, (base_ty, _)) in class.bases.bases.iter().enumerate().rev() {
        let base_ident = make_base_name(class.bases.ident(idx).unwrap());
        // TODO: add visibility specifiers to definitions
        fields.insert(0, parse_quote!(pub #base_ident: #base_ty));
    }

    // add the vtable if there aren't any bases and there are virtuals.
    if class.bases.bases.is_empty() && !class.body.virtuals.is_empty() {
        // push the VTable member
        let generic_args = class.generic_args();
        let vtable_ty = make_vtable_ident(ident);
        fields.insert(
            0,
            parse_quote!(pub vfptr: &'static #vtable_ty #generic_args),
        );
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
fn intercept_default(
    class: &ItemClass,
    attrs: &mut [Attribute],
    additional_bases: &HashMap<Path, Vec<Path>>,
) -> Option<File> {
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

    // generate the implementation. start with a naive implementation
    let fields: Vec<FieldValue> = class
        .body
        .fields
        .iter()
        .filter_map(|field| field.ident.as_ref())
        .cloned()
        .map(|field_name| parse_quote!(#field_name: Default::default()))
        .chain(class.bases.idents().map(|base_ty| {
            let base_ident = make_base_name(base_ty);
            parse_quote!(#base_ident: #base_ty::default())
        }))
        .collect_vec();

    let default_fn = parse_quote! {
        fn default() -> Self {
            Self {
                #(#fields),*
            }
        }
    };
    let generics = &class.generics;
    let generic_args = class.generic_args();
    let ident = &class.ident;
    let [impl_fn, default_fn] = &mut imp::hook_fn(class, default_fn, additional_bases)[..] else {
        unreachable!()
    };
    impl_fn.vis = parse_quote!(pub);
    let output = quote! {
        impl #generics #ident #generic_args {
            #impl_fn
        }

        impl #generics Default for #ident #generic_args {
            #default_fn
        }
    };
    Some(syn::parse(output.into()).expect("failed to generate default implementation"))
}
