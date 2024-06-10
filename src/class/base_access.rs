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
    let generics = &class.generics;
    let generic_args = class.generic_args();
    let base_paths = class
        .bases
        .bases
        .iter()
        .map(|(base, _)| base.clone())
        .collect_vec();
    let base_names = base_paths
        .iter()
        .map(|path| make_base_name(extract_ident(path)))
        .collect_vec();

    syn::parse(
        quote! {
            #(
                impl #generics AsRef<#base_paths> for #ident #generic_args {
                    fn as_ref(&self) -> &#base_paths {
                        &self.#base_names
                    }
                }

                impl #generics AsMut<#base_paths> for #ident #generic_args {
                    fn as_mut(&mut self) -> &mut #base_paths {
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
    let generics = &class.generics;
    let generic_args = class.generic_args();
    let (base_path, _) = class.bases.bases.first()?;
    let base_ident = make_base_name(extract_ident(base_path));

    Some(
        syn::parse(
            quote! {
                impl #generics core::ops::Deref for #ident #generic_args {
                    type Target = #base_path;
                    fn deref(&self) -> &Self::Target {
                        &self.#base_ident
                    }
                }

                impl #generics core::ops::DerefMut for #ident #generic_args {
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        &mut self.#base_ident
                    }
                }
            }
            .into(),
        )
        .expect("failed to generate `Deref` impl"),
    )
}
