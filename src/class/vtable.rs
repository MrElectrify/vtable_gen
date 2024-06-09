use std::collections::BTreeMap;

use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{
    AngleBracketedGenericArguments, Expr, Field, FieldMutability, FieldValue, File, ItemImpl,
    ItemStruct, parse_quote, Path, Visibility,
};
use syn::punctuated::Punctuated;
use syn::token::Comma;

use crate::class::trt::make_virtuals;
use crate::parse::{ItemClass, Virtual};

/// Generates a VTable for the class.
pub fn gen_vtable(class: &ItemClass) -> File {
    let virtuals = sort_virtuals(class);

    // generate the vtable structure
    let vtable = gen_vtable_struct(class, &virtuals);

    // generate the vtable static
    let stc = gen_vtable_static(class, &virtuals);

    syn::parse(
        quote! {
            #vtable
            #stc
        }
        .into(),
    )
    .expect("failed to generate vtable")
}

/// Make the VTable struct identifier.
pub fn make_vtable_struct(ident: &Ident) -> Ident {
    format_ident!("{}VTable", ident)
}

/// Make the VTable static identifier.
pub fn make_vtable_static(ident: &Ident, generics: AngleBracketedGenericArguments) -> Path {
    parse_quote!(#ident :: #generics :: VTBL)
}

/// Generates the default VTable for the class.
fn gen_vtable_static(class: &ItemClass, virtuals: &BTreeMap<usize, Virtual>) -> ItemImpl {
    let class_ident = &class.ident;
    let generic_args = class.generic_args();
    let vis = &class.vis;
    let virtuals_ident = make_virtuals(class_ident);
    let mut body = Punctuated::<FieldValue, Comma>::new();

    if let Some((high_idx, _)) = virtuals.last_key_value() {
        for idx in 0..=*high_idx {
            // either translate the virtual into a function, or generate an unimplemented virtual
            let (ident, expr): (Ident, Expr) = if let Some(virt) = virtuals.get(&idx) {
                let ident = virt.sig.ident.clone();
                let stmt = parse_quote!(<#class_ident #generic_args as #virtuals_ident #generic_args>::#ident);

                (ident, stmt)
            } else {
                let ident = format_ident!("unimpl_{idx}");
                let stmt = parse_quote!(|| unimplemented!());

                (ident, stmt)
            };

            body.push(parse_quote! { #ident: #expr });
        }
    }

    let generics = &class.generics;
    let generic_args = class.generic_args();
    let vtable_struct_ident = make_vtable_struct(class_ident);

    syn::parse(
        quote! {
            impl #generics #class_ident #generic_args {
                #vis const VTBL: #vtable_struct_ident #generic_args = #vtable_struct_ident :: #generic_args {
                    #body
                };
            }
        }
        .into(),
    )
    .expect("failed to generate vtable static")
}

/// Generates the VTable struct for the class.
fn gen_vtable_struct(class: &ItemClass, virtuals: &BTreeMap<usize, Virtual>) -> ItemStruct {
    let vis = &class.vis;
    let vtable_ident = make_vtable_struct(&class.ident);
    let mut body = Punctuated::<Field, Comma>::new();

    if let Some((high_idx, _)) = virtuals.last_key_value() {
        for idx in 0..=*high_idx {
            // either translate the virtual into a function, or generate an unimplemented virtual
            let (virt_ident, virt_ty, attrs) = if let Some(virt) = virtuals.get(&idx) {
                let ident = virt.sig.ident.clone();

                let unsafety = &virt.sig.unsafety;
                let abi = virt.sig.abi.clone().unwrap();
                let args = virt.sig.inputs.clone();
                let output = &virt.sig.output;

                // we need to generate the type from the signature
                let ty = parse_quote!(#unsafety #abi fn(#args) #output);

                (ident, ty, virt.attrs.clone())
            } else {
                let ident = format_ident!("unimpl_{idx}");
                let ty = parse_quote!(fn());
                (ident, ty, vec![])
            };

            body.push(Field {
                attrs,
                vis: Visibility::Inherited,
                mutability: FieldMutability::None,
                ident: Some(virt_ident),
                colon_token: None,
                ty: virt_ty,
            });
        }
    }

    let generics = &class.generics;
    syn::parse(
        quote! {
            #[repr(C)]
            #vis struct #vtable_ident #generics {
                #body
            }
        }
        .into(),
    )
    .expect("failed to generate vtable struct")
}

/// Organizes the virtuals in index-order.
fn sort_virtuals(class: &ItemClass) -> BTreeMap<usize, Virtual> {
    let mut virtuals = BTreeMap::new();
    let mut last_idx = None;
    for virt in class.body.virtuals.iter() {
        let idx = match (&virt.index.idx, &last_idx) {
            (Some(idx), _) => idx.base10_parse().expect("virtual index must be base-10"),
            (None, Some(last_idx)) => *last_idx + 1,
            (None, None) => 0,
        };

        // try to insert the virtual
        if let Some(last_virt) = virtuals.insert(idx, virt.clone()) {
            panic!(
                "virtual {} already occupies index {idx}",
                last_virt.sig.ident
            );
        }

        last_idx = Some(idx);
    }

    virtuals
}
