use itertools::Itertools;
use quote::{format_ident, quote};
use syn::{Attribute, Field, FieldMutability, File, Meta, parse_quote, Token, Visibility};
use syn::punctuated::Punctuated;

use crate::class::make_base_name;
use crate::class::vtable::{make_vtable_ident, make_vtable_static};
use crate::parse::ItemClass;
use crate::util::extract_ident;

/// Generates the base structure.
pub fn gen_struct(class: &ItemClass) -> File {
    let mut attrs = class.attrs.clone();

    let default_impl = intercept_default(class, &mut attrs);

    let vis = &class.vis;
    let ident = &class.ident;
    let generics = &class.generics;

    // add the bases to the fields list
    let mut fields = class.body.fields.clone();
    for (idx, (base_ty, _)) in class.bases.bases.iter().enumerate().rev() {
        let base_ident = make_base_name(class.bases.ident(idx).unwrap());
        fields.insert(0, parse_quote!(#base_ident: #base_ty));
    }

    // add the vtable if there aren't any bases and there are virtuals.
    if class.bases.bases.is_empty() && !class.body.virtuals.is_empty() {
        // push the VTable member
        let generic_args = class.generic_args();
        let vtable_ty = make_vtable_ident(ident);
        fields.insert(
            0,
            Field {
                attrs: vec![],
                vis: Visibility::Inherited,
                mutability: FieldMutability::None,
                ident: Some(format_ident!("vfptr")),
                colon_token: None,
                ty: parse_quote!(&'static #vtable_ty #generic_args),
            },
        )
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
fn intercept_default(class: &ItemClass, attrs: &mut [Attribute]) -> Option<File> {
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
    let generics = &class.generics;
    let generic_args = class.generic_args();
    let vtable_static_ident = make_vtable_static(ident, ident, &generic_args);
    let generic_args = class.generic_args();

    // the secondary base classes
    let secondary_base_idents = class.bases.paths().skip(1).map(extract_ident).collect_vec();
    let secondary_base_params = secondary_base_idents
        .iter()
        .cloned()
        .map(make_base_name)
        .collect_vec();
    let secondary_base_vtable_types = secondary_base_idents
        .iter()
        .cloned()
        .map(make_vtable_ident)
        .collect_vec();
    let secondary_base_statics = secondary_base_idents
        .iter()
        .map(|base_ident| make_vtable_static(ident, base_ident, &generic_args))
        .collect_vec();

    // either delegate the vtable up another level or set it directly
    let vtbl_initializer = if let Some(base_ty) = class.bases.ident(0) {
        let primary_base_ident = make_base_name(base_ty);
        quote! { #primary_base_ident: #base_ty::_default_with_vtable(&vfptr.#primary_base_ident) }
    } else {
        quote! { vfptr }
    };

    let vtable_ty = make_vtable_ident(&class.ident);
    let output = quote! {
        impl #generics #ident #generic_args {
            fn _default_with_vtable(vfptr: &'static #vtable_ty #generic_args,
                #(#secondary_base_params: &'static #secondary_base_vtable_types),*) -> Self {
                Self {
                    #vtbl_initializer,
                    #(#secondary_base_params: #secondary_base_idents::_default_with_vtable(#secondary_base_params),)*
                    #(#arg_names: Default::default()),*
                }
            }
        }

        impl #generics Default for #ident #generic_args {
            fn default() -> Self {
                Self::_default_with_vtable(&#vtable_static_ident, #(&#secondary_base_statics),*)
            }
        }
    };
    Some(syn::parse(output.into()).expect("failed to generate default implementation"))
}
