use std::collections::BTreeMap;

use convert_case::{Case, Casing};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    AngleBracketedGenericArguments, Field, FieldMutability, File, ItemConst, ItemImpl, ItemMacro,
    ItemStruct, parse_quote, Path, PathArguments, Visibility,
};
use syn::punctuated::Punctuated;
use syn::token::Comma;

use crate::class::make_base_name;
use crate::class::trt::make_virtuals;
use crate::parse::{ItemClass, Virtual};
use crate::util::{extract_ident, last_segment};

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
pub fn make_vtable_ident(ident: &Ident) -> Ident {
    format_ident!("{ident}VTable")
}

/// Make the VTable struct identifier.
pub fn make_vtable_macro_ident(ident: &Ident) -> Ident {
    format_ident!("gen_{}_vtable", ident.to_string().to_case(Case::Snake))
}

/// Make the VTable static identifier for a base class. Only used for secondary implementations.
pub fn make_vtable_static(
    ident: &Ident,
    base: &Ident,
    generics: &AngleBracketedGenericArguments,
) -> Path {
    let vtable_ident = format_ident!(
        "VTBL_FOR_{}",
        base.to_string().to_case(Case::ScreamingSnake)
    );
    parse_quote!(#ident :: #generics :: #vtable_ident)
}

/// Extracts generics from an implementor.
fn extract_implementor_generics(class: &ItemClass, base_path: &Path) -> Vec<TokenStream> {
    let class_generics = class.generic_args();

    // extract the angle bracketed_arguments
    let def_generics = match &base_path
        .segments
        .last()
        .expect("expected path segment")
        .arguments
    {
        PathArguments::AngleBracketed(def_generics) => def_generics.clone(),
        _ => parse_quote!(<>),
    };

    // determine the position of each and extract it out of the parent definition
    def_generics
        .args
        .iter()
        .map(|base_arg| {
            class_generics
                .args
                .iter()
                .position(|class_arg| class_arg == base_arg)
                .map(|idx| {
                    let ident = format_ident!("def_generic_{idx}");
                    quote! { $#ident }
                })
                .unwrap_or_else(|| base_arg.to_token_stream())
        })
        .collect()
}

/// Generates a macro that populates the VTable for `class`.
fn gen_vtable_macro(class: &ItemClass, virtuals: &BTreeMap<usize, Virtual>) -> ItemMacro {
    let class_ident = &class.ident;
    let virtuals_ident = make_virtuals(class_ident);
    let mut fields = Vec::new();

    // collect all generic args into descriptors
    let def_generic_arg_idents = class
        .generic_args()
        .args
        .iter()
        .enumerate()
        .map(|(idx, _)| format_ident!("def_generic_{idx}"))
        .collect_vec();

    if let Some((high_idx, _)) = virtuals.last_key_value() {
        for idx in 0..=*high_idx {
            // either translate the virtual into a function, or generate an unimplemented virtual
            let (ident, expr): (Ident, TokenStream) = if let Some(virt) = virtuals.get(&idx) {
                let ident = virt.sig.ident.clone();
                let stmt = quote!(<$implementor_ty as #virtuals_ident <#($#def_generic_arg_idents),*>>::#ident);

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
        let macro_ident = make_vtable_macro_ident(base_ty);
        let base_ident = make_base_name(base_ty);

        // determine the position of each and extract it out of the parent definition
        let base_def_args = extract_implementor_generics(class, base_path);
        fields.insert(
            0,
            parse_quote!(#base_ident: #macro_ident!($implementor_ty, <#(#base_def_args),*>)),
        )
    }

    let macro_ident = make_vtable_macro_ident(class_ident);
    let struct_ident = make_vtable_ident(class_ident);

    let output = quote! {
        #[macro_export]
        macro_rules! #macro_ident {
            // implementor_ty: The type of the implementor.
            // gen_x: The definition generic at position `x`.
            ($implementor_ty:ty, <#($#def_generic_arg_idents: tt),*>) => {
                #struct_ident :: <#($#def_generic_arg_idents),*> {
                    #(#fields),*
                }
            }
        }
    };
    syn::parse(output.into()).expect("failed to generate vtable macro")
}

/// Generates a VTable static for a type.
fn gen_vtable_static_for(
    class_ident: &Ident,
    class_generics: &AngleBracketedGenericArguments,
    vtable_ty: &Ident,
    vis: &Visibility,
    base_generics: &AngleBracketedGenericArguments,
) -> ItemConst {
    let macro_ident = make_vtable_macro_ident(vtable_ty);
    let vtable_static_path = make_vtable_static(class_ident, vtable_ty, &base_generics);
    let vtable_static_ident = extract_ident(&vtable_static_path);
    let vtable_struct_ident = make_vtable_ident(vtable_ty);

    let output = quote! {
        #vis const #vtable_static_ident: #vtable_struct_ident #base_generics =
            #macro_ident!(#class_ident #class_generics, #base_generics);
    };
    syn::parse(output.into())
        .unwrap_or_else(|e| panic!("failed to generate vtable {vtable_ty} for {class_ident}: {e}"))
}

/// Generates the default VTable for the class.
fn gen_vtable_static(class: &ItemClass) -> ItemImpl {
    let class_ident = &class.ident;
    let class_vis = &class.vis;
    let generics = &class.generics;
    let generic_args = class.generic_args();

    // generate the primary vtable
    let mut consts = vec![gen_vtable_static_for(
        class_ident,
        &generic_args,
        class_ident,
        class_vis,
        &generic_args,
    )];

    // generate secondary vtables
    // the secondary base classes
    let secondary_base_types = class.bases.paths().skip(1).collect_vec();
    for secondary_base_type in &secondary_base_types {
        let last_segment = last_segment(secondary_base_type);
        let base_ident = &last_segment.ident;
        let base_generics =
            if let PathArguments::AngleBracketed(def_generics) = &last_segment.arguments {
                def_generics.clone()
            } else {
                parse_quote!(<>)
            };

        consts.push(gen_vtable_static_for(
            class_ident,
            &generic_args,
            base_ident,
            class_vis,
            &base_generics,
        ))
    }

    let output = quote! {
        impl #generics #class_ident #generic_args {
            #(#consts)*
        }
    };
    syn::parse(output.into()).expect("failed to generate vtable static")
}

/// Generates the VTable struct for the class.
fn gen_vtable_struct(class: &ItemClass, virtuals: &BTreeMap<usize, Virtual>) -> ItemStruct {
    let vis = &class.vis;
    let vtable_ident = make_vtable_ident(&class.ident);
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
        let base_vtable_ident = make_vtable_ident(base_ident);
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
            #[derive(Debug)]
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
