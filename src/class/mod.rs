use quote::quote;
use syn::{FnArg, parse_macro_input, parse_quote, PatType};

use crate::parse::{CppDef, ItemClass};

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

    let output = quote! {
        #stct
        #impl_hooks
        #trt
        #vtable
        #bridge
    };
    output.into()
}

/// Standardizes the ABI and signatures for virtuals.
fn standardize_virtuals(class: &mut ItemClass) {
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
                ty: Box::new(parse_quote!(&#mutability #class_ident)),
            });
        } else {
            panic!("virtuals must take `&self` or `&mut self`")
        }
    }
}

// // implement `Deref` and `DerefMut` for the primary field
// fn impl_as_ref(class_name: &Ident, secondary_bases: &[CppClassField]) -> proc_macro2::TokenStream {
//     let names = secondary_bases
//         .iter()
//         .map(|base| base.ident.as_ref().expect("Base classes must be named!"))
//         .collect_vec();
//
//     let tys = secondary_bases.iter().map(|base| &base.ty).collect_vec();
//
//     quote! {
//         #(
//             impl AsRef<#tys> for #class_name {
//                 fn as_ref(&self) -> &#tys {
//                     &self.#names
//                 }
//             }
//
//             impl AsMut<#tys> for #class_name {
//                 fn as_mut(&mut self) -> &mut #tys {
//                     &mut self.#names
//                 }
//             }
//         )*
//     }
// }
//
// /// Implements anything necessary for base classes.
// fn impl_bases(args: &CppClassArgs, fields: &mut FieldsNamed) -> proc_macro2::TokenStream {
//     let bases = args.collect_base_fields();
//     if bases.is_empty() {
//         // extend the struct with an additional field
//         insert_vtbl_field(fields);
//
//         quote! {}
//     } else {
//         // do the things
//         let deref = impl_deref(&args.ident, &bases[0]);
//         let as_ref = impl_as_ref(&args.ident, &bases[1..]);
//
//         quote! {
//             #deref
//             #as_ref
//         }
//     }
// }
//
// // implement `Deref` and `DerefMut` for the primary field
// fn impl_deref(class_name: &Ident, primary_base: &CppClassField) -> proc_macro2::TokenStream {
//     let name = &primary_base
//         .ident
//         .as_ref()
//         .expect("Base classes must be named!");
//     let ty = &primary_base.ty;
//
//     quote! {
//         impl core::ops::Deref for #class_name {
//             type Target = #ty;
//             fn deref(&self) -> &Self::Target {
//                 &self.#name
//             }
//         }
//
//         impl core::ops::DerefMut for #class_name {
//             fn deref_mut(&mut self) -> &mut Self::Target {
//                 &mut self.#name
//             }
//         }
//     }
// }
//
// /// Inserts the `vtbl` field at the beginning of a structure.
// fn insert_vtbl_field(fields: &mut FieldsNamed) {
//     fields.named.insert(
//         0,
//         Field {
//             attrs: vec![],
//             vis: Visibility::Inherited,
//             mutability: FieldMutability::None,
//             ident: Some(Ident::new("_vtbl", Span::call_site())),
//             colon_token: None,
//             ty: Type::from_string("usize").unwrap(),
//         },
//     );
// }
