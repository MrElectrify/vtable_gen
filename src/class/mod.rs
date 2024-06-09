use convert_case::{Case, Casing};
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{FnArg, parse_macro_input, parse_quote, PatType};

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

    // generate the base rust structure
    let stct = stct::gen_struct(&def.class);

    // generate the bridge between the class and its virtuals before standardizing the ABI
    let bridge = bridge::gen_bridge(&def.class);

    // standardize the ABI and signatures for virtuals before passing on the class
    standardize_virtuals(&mut def.class);

    // generate the trait
    let trt = trt::gen_trait(&def.class);

    // generate the VTable structure
    let vtable = vtable::gen_vtable(&def.class);

    // generate implementation hooks
    let impl_hooks = imp::gen_hooks(&def);

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
