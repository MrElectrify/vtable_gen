use std::collections::BTreeMap;

use convert_case::{Case, Casing};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{
    AngleBracketedGenericArguments, Field, FieldMutability, File, ItemImpl, ItemMacro, ItemStruct,
    parse_quote, Path, Visibility,
};
use syn::punctuated::Punctuated;
use syn::token::Comma;

use crate::class::make_base_name;
use crate::class::trt::make_virtuals;
use crate::parse::{ItemClass, Virtual};
use crate::util::extract_ident;

/// Generates a VTable for the class.
pub fn gen_vtable(class: &ItemClass) -> File {
    let virtuals = sort_virtuals(class);

    // generate the vtable structure
    let vtable = gen_vtable_struct(class, &virtuals);

    // generate the macro
    let mcro = gen_vtable_macro(class, &virtuals);

    // generate the vtable static
    let stc = gen_vtable_static(class);

    syn::parse(
        quote! {
            #vtable
            #mcro
            #stc
        }
        .into(),
    )
    .expect("failed to generate vtable")
}

/// Make the VTable struct identifier.
pub fn make_vtable_struct(ident: &Ident) -> Ident {
    format_ident!("{ident}VTable")
}

/// Make the VTable struct identifier.
pub fn make_vtable_macro(ident: &Ident) -> Ident {
    format_ident!("gen_{}_vtable", ident.to_string().to_case(Case::Snake))
}

/// Make the VTable static identifier.
pub fn make_vtable_static(ident: &Ident, generics: AngleBracketedGenericArguments) -> Path {
    parse_quote!(#ident :: #generics :: VTBL)
}

/// Generates a macro that populates the VTable for `class`.
fn gen_vtable_macro(class: &ItemClass, virtuals: &BTreeMap<usize, Virtual>) -> ItemMacro {
    let class_ident = &class.ident;
    let virtuals_ident = make_virtuals(class_ident);
    let mut fields = Vec::new();

    if let Some((high_idx, _)) = virtuals.last_key_value() {
        for idx in 0..=*high_idx {
            // either translate the virtual into a function, or generate an unimplemented virtual
            let (ident, expr): (Ident, TokenStream) = if let Some(virt) = virtuals.get(&idx) {
                let ident = virt.sig.ident.clone();
                let stmt = quote!(<$implementor_ty <$($implementor_ty_generics),*> as #virtuals_ident <$($def_generics),*>>::#ident);

                (ident, stmt)
            } else {
                let ident = format_ident!("unimpl_{idx}");
                let stmt = quote!(|| unimplemented!());

                (ident, stmt)
            };

            fields.push(quote! { #ident: #expr });
        }
    }

    // generate the base vtable
    if let Some(base_path) = class.bases.path(0) {
        let base_ty = extract_ident(base_path);
        let def_generics = &base_path
            .segments
            .last()
            .expect("expected path segment")
            .arguments;
        let macro_ident = make_vtable_macro(base_ty);
        let base_ident = make_base_name(base_ty);

        fields.insert(
            0,
            parse_quote!(#base_ident: #macro_ident!($implementor_ty <$($implementor_ty_generics),*>, #def_generics)),
        )
    }

    let macro_ident = make_vtable_macro(class_ident);
    let struct_ident = make_vtable_struct(class_ident);

    let output = quote! {
        #[macro_export]
        macro_rules! #macro_ident {
            // implementor_ty: The type of the implementor.
            // implementor_ty_generics: The generic arguments of the implementor.
            // def_generics: The generic arguments named in the base type member.
            ($implementor_ty:ident <$($implementor_ty_generics:tt),*>, <$($def_generics:tt),*>) => {
                #struct_ident :: <$($def_generics),*> {
                    #(#fields),*
                }
            }
        }
    };
    syn::parse(output.into()).expect("failed to generate vtable macro")
}

/// Generates the default VTable for the class.
fn gen_vtable_static(class: &ItemClass) -> ItemImpl {
    let class_ident = &class.ident;
    let vis = &class.vis;
    let generics = &class.generics;
    let generic_args = class.generic_args();
    let macro_ident = make_vtable_macro(class_ident);
    let vtable_struct_ident = make_vtable_struct(class_ident);

    let output = quote! {
        impl #generics #class_ident #generic_args {
            #vis const VTBL: #vtable_struct_ident #generic_args =
                #macro_ident!(#class_ident #generic_args, #generic_args);
        }
    };
    syn::parse(output.into()).expect("failed to generate vtable static")
}

/// Generates the VTable struct for the class.
fn gen_vtable_struct(class: &ItemClass, virtuals: &BTreeMap<usize, Virtual>) -> ItemStruct {
    let vis = &class.vis;
    let vtable_ident = make_vtable_struct(&class.ident);
    let mut fields = Punctuated::<Field, Comma>::new();

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

            fields.push(Field {
                attrs,
                vis: Visibility::Inherited,
                mutability: FieldMutability::None,
                ident: Some(virt_ident),
                colon_token: None,
                ty: virt_ty,
            });
        }
    }

    // add the base VTable if there is one
    if let Some(base_path) = class.bases.path(0) {
        let base_ident = extract_ident(base_path);
        let base_vtable_ident = make_vtable_struct(base_ident);
        let base_args = &base_path
            .segments
            .last()
            .expect("expected path segment")
            .arguments;

        fields.insert(
            0,
            Field {
                attrs: vec![],
                vis: Visibility::Inherited,
                mutability: FieldMutability::None,
                ident: Some(make_base_name(base_ident)),
                colon_token: None,
                ty: parse_quote!(#base_vtable_ident #base_args),
            },
        )
    }

    let generics = &class.generics;
    syn::parse(
        quote! {
            #[repr(C)]
            #vis struct #vtable_ident #generics {
                #fields
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
