use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, ItemImpl, parse_quote, Pat};

use crate::class::vtable::make_vtable_struct;
use crate::parse::ItemClass;

/// Generates a bridge between a class and its virtuals.
pub fn gen_bridge(class: &ItemClass) -> ItemImpl {
    let ident = &class.ident;
    let vtable_ident = make_vtable_struct(&class.ident);

    // generate direct functions
    let mut fns: Vec<ItemFn> = Vec::new();
    for virt in class.body.virtuals.iter() {
        let arg_names: Vec<Ident> = virt
            .sig
            .inputs
            .iter()
            .map(|arg| match arg {
                FnArg::Receiver(_) => format_ident!("self"),
                FnArg::Typed(ty) => {
                    if let Pat::Ident(ident) = &*ty.pat {
                        ident.ident.clone()
                    } else {
                        panic!("virtual args must have identifiers")
                    }
                }
            })
            .collect();

        let attrs = &virt.attrs;
        let vis = &virt.vis;
        let unsafety = &virt.sig.unsafety;
        let ident = &virt.sig.ident;
        let args = &virt.sig.inputs;
        let output = &virt.sig.output;

        fns.push(parse_quote! {
            #(#attrs)*
            #vis #unsafety fn #ident (#args) #output {
                let vtbl = unsafe { &*(self.vfptr as *const #vtable_ident) };
                (vtbl.#ident)(#(#arg_names),*)
            }
        });
    }

    syn::parse(
        quote! {
            impl #ident {
                #(#fns)*
            }
        }
        .into(),
    )
    .expect("failed to generate bridges")
}
