use convert_case::{Case, Casing};
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{FnArg, GenericParam, parse_macro_input, parse_quote, Path, PatType, Token};
use syn::punctuated::Punctuated;

use crate::parse::{CppDef, ItemClass};

mod base_access;
mod bridge;
mod imp;
mod stct;
mod trt;
mod vtable;

/// Generates the Rust Struct, VTable struct and Virtuals struct.
pub fn cpp_class_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut def = parse_macro_input!(input as CppDef);

    // extract `gen_base`
    let additional_bases = extract_additional_bases(&mut def.class);

    // enforces static trait bounds (required for VTable)
    enforce_static(&mut def.class);

    // generate the base rust structure
    let stct = stct::gen_struct(&def.class, &additional_bases);

    // generate the bridge between the class and its virtuals before standardizing the ABI
    let bridge = bridge::gen_bridge(&def.class);

    // standardize the ABI and signatures for virtuals before passing on the class
    standardize_virtuals(&mut def.class);

    // generate the trait
    let trt = trt::gen_trait(&def.class);

    // generate the VTable structure
    let vtable = vtable::gen_vtable(&def.class, &additional_bases);

    // generate implementation hooks
    let impl_hooks = imp::gen_hooks(&def, &additional_bases);

    // generate access helpers
    let access_helpers = base_access::gen_base_helpers(&def.class);

    let output = quote! {
        #stct
        #impl_hooks
        #trt
        #vtable
        #bridge
        #access_helpers
    };
    output.into()
}

/// Enforces that each trait parameter is static.
fn enforce_static(class: &mut ItemClass) {
    for generic in class.generics.params.iter_mut() {
        if let GenericParam::Type(ty) = generic {
            ty.bounds.push(parse_quote!('static))
        }
    }
}

/// Extracts additional bases that are explicitly listed.
fn extract_additional_bases(class: &mut ItemClass) -> Vec<Path> {
    // see if there's a `derive` attribute
    let Some(gen_base_idx) = class
        .attrs
        .iter()
        .position(|attr| attr.path().is_ident("gen_base"))
    else {
        return Vec::new();
    };
    let gen_base_attr = class.attrs.remove(gen_base_idx);

    // parse out bases
    gen_base_attr
        .parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)
        .expect("Failed to parse repr")
        .iter()
        .cloned()
        .collect()
}

/// Makes the base identifier for a type.
fn make_base_name(ident: &Ident) -> Ident {
    format_ident!("base_{}", ident.to_string().to_case(Case::Snake))
}

/// Standardizes the ABI and signatures for virtuals.
fn standardize_virtuals(class: &mut ItemClass) {
    let generic_args = class.generic_args();
    for virt in class.body.virtuals.iter_mut() {
        if virt.sig.abi.is_none() {
            virt.sig.abi = parse_quote!(extern "C");
        }

        // if the first arg is `self`, replace it with the type
        let args = &mut virt.sig.inputs;
        if let Some(FnArg::Receiver(receiver)) = args.first().cloned() {
            let class_ident = &class.ident;
            let mutability = receiver.mutability;
            *args.first_mut().unwrap() = FnArg::Typed(PatType {
                attrs: vec![],
                pat: Box::new(parse_quote!(this)),
                colon_token: Default::default(),
                ty: Box::new(parse_quote!(&#mutability #class_ident #generic_args)),
            });
        } else {
            panic!("virtuals must take `&self` or `&mut self`")
        }
    }
}
