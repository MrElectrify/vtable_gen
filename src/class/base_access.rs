use itertools::Itertools;
use quote::quote;
use syn::File;

use crate::class::make_base_name;
use crate::parse::ItemClass;
use crate::util::extract_ident;

/// Implements anything necessary for base classes.
pub fn gen_base_helpers(class: &ItemClass) -> File {
    // do the things
    let deref = impl_deref(class);
    let as_ref = impl_as_ref(class);

    syn::parse(
        quote! {
            #deref
            #as_ref
        }
        .into(),
    )
    .expect("failed to generate bases")
}

// implement `AsRef` for all bases.
fn impl_as_ref(class: &ItemClass) -> File {
    let ident = &class.ident;
    let base_idents = class
        .bases
        .bases
        .iter()
        .map(|(base, _)| extract_ident(base).clone())
        .collect_vec();
    let base_names = base_idents.iter().map(make_base_name).collect_vec();

    syn::parse(
        quote! {
            #(
                impl AsRef<#base_idents> for #ident {
                    fn as_ref(&self) -> &#base_idents {
                        &self.#base_names
                    }
                }

                impl AsMut<#base_idents> for #ident {
                    fn as_mut(&mut self) -> &mut #base_idents {
                        &mut self.#base_names
                    }
                }
            )*
        }
        .into(),
    )
    .expect("failed to generate `AsRef` impl")
}

// implement `Deref` and `DerefMut` for the primary base.
fn impl_deref(class: &ItemClass) -> Option<File> {
    let ident = &class.ident;
    let base_ident = class.bases.ident(0)?;
    let name = make_base_name(base_ident);

    Some(
        syn::parse(
            quote! {
                impl core::ops::Deref for #ident {
                    type Target = #base_ident;
                    fn deref(&self) -> &Self::Target {
                        &self.#name
                    }
                }

                impl core::ops::DerefMut for #ident {
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        &mut self.#name
                    }
                }
            }
            .into(),
        )
        .expect("failed to generate `Deref` impl"),
    )
}
