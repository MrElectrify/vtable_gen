use convert_case::{Case, Casing};
use darling::FromMeta;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{
    File, FnArg, GenericArgument, GenericParam, parse_macro_input, parse_quote, Path, PathArguments,
    PatType, Type,
};

use crate::class::extractor::AttributeExtractor;
use crate::class::gen_vtable::GenVTable;
use crate::class::generic_base::GenericBase;
use crate::class::secondary_base::SecondaryBase;
use crate::parse::{CppDef, ItemClass};
use crate::util::{extract_ident, last_segment_mut, remove_punctuated};

mod base_access;
mod bridge;
mod extractor;
mod gen_vtable;
mod generic_base;
mod imp;
mod secondary_base;
mod stct;
mod trt;
mod vtable;

const BASE_PREFIX: Option<&str> = option_env!("VTABLE_PREFIX");

/// Returns the base prefix of the class.
pub fn base_prefix() -> TokenStream {
    if let Some(base) = BASE_PREFIX
        .map(Path::from_string)
        .transpose()
        .expect("failed to parse base prefix")
    {
        quote!(#base::)
    } else {
        TokenStream::default()
    }
}

/// Generates the Rust Struct, VTable struct and Virtuals struct.
pub fn cpp_class_impl(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut def = parse_macro_input!(input as CppDef);
    let mut output = proc_macro::TokenStream::default();

    // see if there's a generic base replacement
    if let Some(base_collections) = GenericBase::extract(&mut def.class) {
        for generic_bases in base_collections {
            let mut def = def.clone();

            for generic_base in generic_bases {
                // rename the definition
                def.class.ident =
                    format_ident!("{}_{}", def.class.ident, extract_ident(&generic_base.repl));

                // find and rename the base
                if let Some(base) = def
                    .class
                    .bases
                    .bases
                    .iter_mut()
                    .map(|(path, _)| path)
                    .find(|path| extract_ident(path) == &generic_base.base)
                {
                    *base = generic_base.repl.clone();
                }

                // remove the generic
                def.class.generics.params =
                    remove_punctuated(&def.class.generics.params, |param| match param {
                        GenericParam::Type(ty) => ty.ident != generic_base.base,
                        _ => true,
                    });

                // remove the generics from the bases
                for (base, _) in &mut def.class.bases.bases {
                    let last_segment = last_segment_mut(base);
                    if let PathArguments::AngleBracketed(args) = &mut last_segment.arguments {
                        args.args = remove_punctuated(&args.args, |arg| match arg {
                            GenericArgument::Type(Type::Path(ty)) => {
                                if extract_ident(&ty.path) == &generic_base.base {
                                    last_segment.ident = format_ident!(
                                        "{}_{}",
                                        last_segment.ident,
                                        extract_ident(&generic_base.repl)
                                    );
                                    return false;
                                }

                                true
                            }
                            _ => true,
                        });
                    }
                }
            }

            let def = generate_class(def.clone());
            output.extend([proc_macro::TokenStream::from(def.into_token_stream())]);
        }
    } else {
        let def = generate_class(def);
        output.extend([proc_macro::TokenStream::from(def.into_token_stream())]);
    }

    output
}

fn generate_class(mut def: CppDef) -> File {
    // extract `gen_base`
    let additional_bases = SecondaryBase::extract(&mut def.class).unwrap_or_default();

    // extract `gen_vtable`
    let gen_vtable = GenVTable::extract(&mut def.class);

    // enforces static trait bounds (required for VTable)
    enforce_static(&mut def.class);

    // generate the base rust structure
    let stct = stct::gen_struct(&def.class, &additional_bases);

    // generate the bridge between the class and its virtuals before standardizing the ABI
    let bridge = bridge::gen_bridge(&def.class);

    // standardize the ABI and signatures for virtuals before passing on the class
    standardize_virtuals(&mut def.class);

    // generate the trait
    let trt = if gen_vtable.is_some() {
        Some(trt::gen_trait(&def.class))
    } else {
        None
    };

    // generate the VTable structure
    let vtable = vtable::gen_vtable(&def.class, &additional_bases, gen_vtable.is_some());

    // generate implementation hooks
    let impl_hooks = imp::gen_hooks(&def, &additional_bases);

    // generate access helpers
    let access_helpers = base_access::gen_base_helpers(&def.class);

    let output = quote! {
        #[allow(non_camel_case_types)]
        #stct
        #impl_hooks
        #[allow(non_camel_case_types)]
        #trt
        #vtable
        #bridge
        #access_helpers
    };
    syn::parse(output.into()).expect("failed to generate class")
}

/// Enforces that each trait parameter is static.
fn enforce_static(class: &mut ItemClass) {
    for generic in class.generics.params.iter_mut() {
        if let GenericParam::Type(ty) = generic {
            ty.bounds.push(parse_quote!('static))
        }
    }
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
