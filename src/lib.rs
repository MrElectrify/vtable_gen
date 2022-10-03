//! # VTable Gen
//!
//! This crate provides macros to generate C++-ABI VTables by defining the structure and vtable layout.
//! It also supports VTable inheritance and basic class inheritance.
//!
//! # Examples
//!
//! Check out `tests.rs`, which is pretty self-explanatory.
//!
//! # Usage
//!
//! ## Base Structs
//! - Define a structure that contains virtual functions
//! - Define a structure for the VTable that exactly matches the name of the structure it belongs to,
//! followed by `VTable` exactly. Example:
//! ```rs
//! struct Foo {}
//! struct FooVTable {}
//! ```
//! - Mark both the VTable and structure with `#[gen_vtable]`. Any function pointers you include in
//! the VTable struct will require implementation in an automatically-generated `<name>Virtuals` trait.
//! Complete Example:
//! ```rs
//! #[gen_vtable]
//! struct Foo {}
//! #[gen_vtable]
//! struct FooVTable {
//!     foo: extern "C" fn(this: &Foo) -> u32;
//! }
//!
//! impl FooVirtuals for Foo {
//!     extern "C" fn foo(this: &Foo) -> u32 { todo!() }
//! }
//! ```
//!
//! ## Derived Structs
//! - Define structures exactly as with base structures
//! - Include the attribute `base`. Example:
//! ```rs
//! #[gen_vtable]
//! struct Foo {}
//! #[gen_vtable]
//! struct FooVTable {}
//!
//! #[gen_vtable(base = "Foo")]
//! struct Bar {}
//! #[gen_vtable(base = "Foo")]
//! struct BarVTable {}
//! ```
//!
//! ## Constructing Structs with VTables
//!
//! Constructing structs with VTables is easy. If the struct is default-able, simply derive
//! `DefaultVTable` instead of `Default`. This will `impl Default`. If the struct isn't default-able,
//! define some function `fn new(/* args */) -> Self`. Mark the function with `new_with_vtable`,
//! supplying base structs if necessary as in `Derived Structs`. For the compiler to know the type,
//! you must either explicitly replace `Self` as the return type with the type itself, or specify
//! `self_type`. Here's a verbose example:
//!
//! ```rs
//! // ...
//! impl Bar {
//!     #[new_with_vtable(self_type = "Bar")]
//!     fn new(baz: u32) -> Self {
//!         Self { baz }
//!     }
//! }
//! ```
//!
//! which is also equivalent to
//!
//! ```rs
//! // ...
//! impl Bar {
//!     #[new_with_vtable]
//!     fn new(baz: u32) -> Bar {
//!         Self { baz }
//!     }
//! }
//! ```
//!
//! If there is a base struct that requires its `new` function to be called, you will have to also
//! explicitly initialize a `base_with_vtbl` member with the `new` constructor of the child type.
//! For example:
//!
//! ```rs
//! // ...
//! impl Bar {
//!     #[new_with_vtable(base = "Foo", self_type = "Bar")]
//!     fn new(baz: u32) -> Self {
//!         Self {
//!             base_with_vtable: Foo::new(123),
//!             baz
//!         }
//!     }
//! }
//! ```
//!
//! ## Overriding Functions
//!
//! Overriding functions is easy. Because all functions are defined in Traits, one can specify for the
//! compiler to not generate implementations for base struct `Virtuals` with the argument `no_base_trait_impl`
//! on the VTable (or both for symmetry :)).
//! Example:
//!
//! ```rs
//! // ...
//! #[gen_vtable(base = "Foo", no_base_trait_impl)]
//! struct BarVTable {}
//!
//! // ...
//! impl FooVirtuals for Bar {
//!     extern "C" fn some_fn(this: &Foo) {
//!         // ...
//!     }
//! }
//! ```
//!
//! The only caveat is you will have to implement *all* base traits.
//!
//! # Known Limitations
//! - `vtable_gen` currently does not support generic structs. This is a trivial addition, however, and
//! will likely be added in the future

use darling::{FromDeriveInput, FromMeta};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::__private::TokenStream2;
use syn::parse::{Parse, Parser};
use syn::{
    parse_macro_input, Abi, Attribute, AttributeArgs, BareFnArg, Data, DataStruct, DeriveInput,
    Expr, ExprStruct, Field, FieldValue, Fields, FieldsNamed, FnArg, Ident, ItemFn, LitStr, Local,
    Member, Meta, MetaList, NestedMeta, Pat, PatIdent, PathSegment, ReturnType, Stmt, Type,
    Visibility,
};

#[derive(FromMeta)]
struct GenVTableAttributes {
    #[darling(default)]
    base: Option<Type>,
    #[darling(default)]
    no_base_trait_impl: bool,
}

// to make it work
impl FromDeriveInput for GenVTableAttributes {
    fn from_derive_input(__di: &DeriveInput) -> darling::Result<Self> {
        let mut __errors = darling::Error::accumulator();
        let mut base: (bool, Option<Option<Type>>) = (false, None);
        let mut no_base_trait_impl: (bool, Option<bool>) = (false, None);
        let mut __fwd_attrs: Vec<Attribute> = Vec::new();
        for __attr in &__di.attrs {
            match ::darling::export::ToString::to_string(&__attr.path.clone().into_token_stream())
                .as_str()
            {
                "gen_vtable" => match darling::util::parse_attribute_to_meta_list(__attr) {
                    Ok(__data) => {
                        if __data.nested.is_empty() {
                            continue;
                        }
                        let __items = &__data.nested;
                        for __item in __items {
                            if let NestedMeta::Meta(ref __inner) = *__item {
                                let __name = ::darling::util::path_to_string(__inner.path());
                                match __name.as_str() {
                                    "base" => {
                                        if !base.0 {
                                            base = (
                                                true,
                                                __errors.handle(
                                                    ::darling::FromMeta::from_meta(__inner)
                                                        .map_err(|e| {
                                                            e.with_span(&__inner).at("base")
                                                        }),
                                                ),
                                            );
                                        } else {
                                            __errors.push(
                                                darling::Error::duplicate_field("base")
                                                    .with_span(&__inner),
                                            );
                                        }
                                    }
                                    "no_base_trait_impl" => {
                                        if !no_base_trait_impl.0 {
                                            no_base_trait_impl = (
                                                true,
                                                __errors.handle(
                                                    FromMeta::from_meta(__inner).map_err(|e| {
                                                        e.with_span(&__inner)
                                                            .at("no_base_trait_impl")
                                                    }),
                                                ),
                                            );
                                        } else {
                                            __errors.push(
                                                darling::Error::duplicate_field(
                                                    "no_base_trait_impl",
                                                )
                                                .with_span(&__inner),
                                            );
                                        }
                                    }
                                    __other => {
                                        __errors.push(
                                            darling::Error::unknown_field_with_alts(
                                                __other,
                                                &["base", "no_base_trait_impl"],
                                            )
                                            .with_span(__inner),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Err(__err) => {
                        __errors.push(__err);
                    }
                },
                _ => continue,
            }
        }
        __errors.finish()?;
        Ok(GenVTableAttributes {
            base: match base.1 {
                Some(__val) => __val,
                None => ::darling::export::Default::default(),
            },
            no_base_trait_impl: match no_base_trait_impl.1 {
                Some(__val) => __val,
                None => ::darling::export::Default::default(),
            },
        })
    }
}

/// Generates a VTable member for a struct, or a trait if the struct is a vtable. See examples.
#[proc_macro_attribute]
pub fn gen_vtable(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as AttributeArgs);
    let mut input = parse_macro_input!(input as DeriveInput);
    let attr = GenVTableAttributes::from_list(&attr).unwrap();

    // add `#[repr(C)]`
    add_repr_c(&mut input);

    let base_name = match &attr.base {
        Some(Type::Path(p)) if p.path.get_ident().is_some() => Some(p.path.get_ident().unwrap()),
        None => None,
        _ => panic!("Base structs must be a name!"),
    };

    let base_name = base_name.map(|base_name| base_name.to_string());
    let base_name = base_name.as_ref().map(|base_name| base_name.as_ref());

    // parse out the base string
    let struct_name = input.ident.to_string();
    let (is_vtable, struct_name) = if struct_name.ends_with("VTable") {
        (true, &struct_name[..struct_name.len() - 6])
    } else {
        (false, struct_name.as_ref())
    };

    // only generate for vtables
    let gen = if is_vtable {
        gen_vtable_impl(&mut input, &attr, struct_name, base_name)
    } else {
        // simply add the "base" member
        add_vtable_or_base_field(&mut input, base_name, false);
        quote! {}
    };
    let res = quote! {
        #input
        #gen
    };
    res.into()
}

fn has_repr_c(derive_input: &DeriveInput) -> bool {
    // Look for existing #[repr(C)] variants, e.g.,
    // #[repr(C)]
    // #[repr(C, packed(4))]

    let has = |meta: &Meta, ident| meta.path().get_ident().map_or(false, |i| i == ident);

    derive_input
        .attrs
        .iter()
        .filter_map(|a| {
            a.parse_meta()
                .ok()
                .filter(|meta| has(meta, "repr"))
                .and_then(|meta| match meta {
                    Meta::List(MetaList { nested, .. }) => Some(nested),
                    _ => None,
                })
        })
        .flatten()
        .any(|n| match n {
            NestedMeta::Meta(meta) => has(&meta, "C"),
            _ => false,
        })
}

fn add_repr_c(derive_input: &mut DeriveInput) {
    if has_repr_c(derive_input) {
        return;
    }

    let mut repr_c = Attribute::parse_outer
        .parse2(quote! { #[repr(C)] })
        .expect("internal macro error with ill-formed #[repr(C)]");

    derive_input.attrs.append(&mut repr_c);
}

fn gen_vtable_impl(
    input: &mut DeriveInput,
    attr: &GenVTableAttributes,
    struct_name: &str,
    base_name: Option<&str>,
) -> TokenStream2 {
    add_vtable_or_base_field(input, base_name, true);

    let trait_impl = gen_vtable_trait(input, struct_name, base_name, attr.no_base_trait_impl);

    quote! {
        #trait_impl
    }
}

fn add_vtable_or_base_field(input: &mut DeriveInput, base_name: Option<&str>, is_vtable: bool) {
    // base vtables have no extra fields
    if is_vtable && base_name.is_none() {
        return;
    }

    let field_ty_name = if let Some(base_name) = base_name {
        let mut field_ty_name = base_name.to_owned();
        if is_vtable {
            field_ty_name.push_str("VTable");
        }
        field_ty_name
    } else {
        "usize".to_owned()
    };
    let field_ty_name = Ident::new(&field_ty_name, Span::call_site());
    let field_name = if is_vtable {
        if base_name.is_some() {
            "base"
        } else {
            unreachable!()
        }
    } else if base_name.is_some() {
        "base_with_vtbl"
    } else {
        "vtbl"
    };
    let field_name = Ident::new(field_name, Span::call_site());

    match &mut input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named: fields, .. }),
            ..
        }) => {
            // ensure they don't already have this field
            if fields
                .first()
                .map(|first_field| {
                    first_field
                        .ident
                        .as_ref()
                        .map(|ident| ident != &field_name)
                        .unwrap_or(true)
                })
                .unwrap_or(true)
            {
                fields.insert(
                    0,
                    Field::parse_named
                        .parse2(quote! { pub #field_name: #field_ty_name })
                        .expect("Failed to create field in struct"),
                );
            }
        }
        _ => panic!("#[derive(GenVTable)] can only be used on a struct with named fields!"),
    }
}

fn gen_vtable_trait(
    input: &mut DeriveInput,
    struct_name: &str,
    base_name: Option<&str>,
    no_base_trait_impl: bool,
) -> TokenStream2 {
    let trait_name = String::from(struct_name) + "Virtuals";
    let trait_name = Ident::new(&trait_name, Span::call_site());
    let vis = &input.vis;

    let fields = match &mut input.data {
        Data::Struct(stct) => &mut stct.fields,
        _ => panic!("#[derive(GenVTable)] can only be used on a struct!"),
    };

    let skip_n = usize::from(base_name.is_some());
    let trait_methods: Vec<(LitStr, &Ident, Vec<&BareFnArg>, TokenStream2)> = fields
        .iter_mut()
        .skip(skip_n)
        .map(|field| match &mut field.ty {
            Type::BareFn(bare_fn) => {
                // add `extern "C"` if it is missing
                if bare_fn.abi.is_none() {
                    bare_fn.abi = Some(Abi::parse.parse2(quote! { extern "C" }).unwrap());
                }
                let abi = bare_fn
                    .abi
                    .as_ref()
                    .and_then(|abi| abi.name.clone())
                    .unwrap();
                let name = field.ident.as_ref().expect("Only named fields are allowed");
                let args: Vec<&BareFnArg> = bare_fn.inputs.iter().collect();
                let output = match &bare_fn.output {
                    ReturnType::Default => quote! {},
                    ReturnType::Type(_, ty) => quote! {
                        -> #ty
                    },
                };

                (abi, name, args, output)
            }
            _ => {
                panic!("The members of a VTable can only be functions!");
            }
        })
        .collect();
    let abi: Vec<&LitStr> = trait_methods.iter().map(|(abi, _, _, _)| abi).collect();
    let name: Vec<&Ident> = trait_methods.iter().map(|(_, name, _, _)| *name).collect();
    let args: Vec<&Vec<&BareFnArg>> = trait_methods.iter().map(|(_, _, args, _)| args).collect();
    let arg_names: Vec<Vec<&Ident>> = args
        .iter()
        .map(|args| {
            args.iter()
                .map(|arg| &arg.name.as_ref().unwrap().0)
                .collect()
        })
        .collect();
    let output: Vec<&TokenStream2> = trait_methods
        .iter()
        .map(|(_, _, _, output)| output)
        .collect();

    let inherit_base: TokenStream2 = if let Some(base_name) = base_name {
        let base_virtuals_name = base_name.to_owned() + "Virtuals";
        let base_virtuals_name = Ident::new(&base_virtuals_name, Span::call_site());

        quote! {
            : #base_virtuals_name
        }
    } else {
        quote! {}
    };

    let mut impl_virtuals_macro_name = "impl_".to_owned();
    impl_virtuals_macro_name.push_str(struct_name);
    impl_virtuals_macro_name.push_str("Virtuals_for");
    let impl_virtuals_macro_name = Ident::new(&impl_virtuals_macro_name, Span::call_site());

    let mut impl_vtable_macro_name = "impl_".to_owned();
    impl_vtable_macro_name.push_str(struct_name);
    impl_vtable_macro_name.push_str("VTable_for");
    let impl_vtable_macro_name = Ident::new(&impl_vtable_macro_name, Span::call_site());

    let struct_name = Ident::new(struct_name, Span::call_site());

    let impl_base_virtuals_macro = if let Some(base_name) = base_name {
        let mut impl_base_virtuals_macro_name = "impl_".to_owned();
        impl_base_virtuals_macro_name.push_str(base_name);
        impl_base_virtuals_macro_name.push_str("Virtuals_for");
        let impl_base_virtuals_macro_name =
            Ident::new(&impl_base_virtuals_macro_name, Span::call_site());

        quote! {
            #impl_base_virtuals_macro_name!($ty);
        }
    } else {
        quote! {}
    };

    let visibility_export_stmt = match vis {
        Visibility::Crate(_) | Visibility::Public(_) | Visibility::Restricted(_) => {
            quote! { #[macro_export] }
        }
        _ => quote! { #[allow(unused_macros)] },
    };

    let impl_virtuals_macro = quote! {
        #visibility_export_stmt
        macro_rules! #impl_virtuals_macro_name {
            ($ty:ty) => {
                #impl_base_virtuals_macro

                impl #trait_name for $ty {
                    #(extern #abi fn #name(#(#args),*) #output {
                        #struct_name::#name(#(#arg_names),*)
                    })*
                }
            }
        }
    };

    let vtable_name = &input.ident;

    let base_impl = if let Some(base_name) = base_name {
        let mut impl_base_vtable_macro_name = "impl_".to_owned();
        impl_base_vtable_macro_name.push_str(base_name);
        impl_base_vtable_macro_name.push_str("VTable_for");
        let impl_base_virtuals_macro_name =
            Ident::new(&impl_base_vtable_macro_name, Span::call_site());

        quote! {
            base: #impl_base_virtuals_macro_name!($ty),
        }
    } else {
        quote! {}
    };

    let impl_vtable_macro = quote! {
        #visibility_export_stmt
        macro_rules! #impl_vtable_macro_name {
            ($ty:ty) => {
                #vtable_name {
                    #base_impl
                    #(#name: <$ty as #trait_name>::#name),*
                }
            }
        }
    };

    let vtable_constant_name = struct_name.to_string().to_uppercase() + "_VTBL";
    let vtable_constant_name = Ident::new(&vtable_constant_name, Span::call_site());

    let base_trait_impl = if !no_base_trait_impl {
        if let Some(base_name) = base_name {
            let mut impl_base_virtuals_macro_name = "impl_".to_owned();
            impl_base_virtuals_macro_name.push_str(base_name);
            impl_base_virtuals_macro_name.push_str("Virtuals_for");
            let impl_base_virtuals_macro_name =
                Ident::new(&impl_base_virtuals_macro_name, Span::call_site());

            quote! {
                #impl_base_virtuals_macro_name!(#struct_name);
            }
        } else {
            quote! {}
        }
    } else {
        quote! {}
    };

    quote! {
        #vis trait #trait_name #inherit_base {
            #(extern #abi fn #name(#(#args),*) #output;)*
        }

        // implement the macro which generates the trait passthrough
        #impl_virtuals_macro

        // implement the macro which creates the vtable
        #impl_vtable_macro

        // generate the static vtable
        #vis static #vtable_constant_name: #vtable_name = #impl_vtable_macro_name!(#struct_name);

        // implement the base trait, if required
        #base_trait_impl
    }
}

/// Implements `std::Default` for a struct containing a VTable
#[proc_macro_derive(DefaultVTable, attributes(gen_vtbl))]
pub fn default_vtable(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    let attr = GenVTableAttributes::from_derive_input(&derive_input).unwrap();
    let struct_name = &derive_input.ident;
    let vis = &derive_input.vis;

    let vtable_name = struct_name.to_string() + "VTable";
    let vtable_name = Ident::new(&vtable_name, Span::call_site());

    let vtable_constant_name = struct_name.to_string().to_uppercase() + "_VTBL";
    let vtable_constant_name = Ident::new(&vtable_constant_name, Span::call_site());

    let field_names: Vec<&Ident> = match &derive_input.data {
        Data::Struct(stct) => &stct.fields,
        _ => panic!("#[derive(GenVTable)] can only be used on a struct!"),
    }
    .iter()
    .map(|field| field.ident.as_ref().expect("Only named fields are allowed"))
    .filter(|ident| ident != &"vtbl" && ident != &"base_with_vtable")
    .collect();

    let vtbl_initializer = if let Some(base) = &attr.base {
        let base_name = match base {
            Type::Path(p) if p.path.get_ident().is_some() => p.path.get_ident().unwrap(),
            _ => panic!("Base structs must be a name!"),
        };

        quote! { base_with_vtbl: #base_name::__with_vtbl(vtbl) }
    } else {
        quote! { vtbl }
    };

    let res = quote! {
        impl #struct_name {
            #[doc(hidden)]
            #vis fn __with_vtbl(vtbl: usize) -> Self {
                Self {
                    #vtbl_initializer,
                    #(#field_names: ::std::default::Default::default(),)*
                }
            }
        }

        impl ::std::default::Default for #struct_name {
            #[allow(clippy::needless_update)]
            fn default() -> Self {
                Self::__with_vtbl(&#vtable_constant_name as *const #vtable_name as usize)
            }
        }
    };

    res.into()
}

#[derive(FromMeta)]
struct NewWithVTableAttributes {
    #[darling(default)]
    base: Option<Type>,
    #[darling(default)]
    self_type: Option<Type>,
}

/// Sets the vtable and allows for passthrough of the vtable from further-derived structs.
#[proc_macro_attribute]
pub fn new_with_vtable(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attr = parse_macro_input!(attr as AttributeArgs);
    let attr = NewWithVTableAttributes::from_list(&attr).unwrap();
    let mut fn_input = parse_macro_input!(input as ItemFn);

    let struct_name = if let Some(self_type) = &attr.self_type {
        match &self_type {
            Type::Path(path) => path.path.get_ident().unwrap(),
            _ => panic!("`self_type` must be a path to Self"),
        }
    } else {
        match &fn_input.sig.output {
            ReturnType::Type(_, typ) => {
                match &**typ {
                    Type::Path(path) if path.path.get_ident().unwrap() != "Self" => {
                        path.path.get_ident().unwrap()
                    },
                    _ => panic!("`new` must return the explicit type of `Self`, or be specified with `self_type`")
                }
            }
            _ => panic!(
                "`new` must return the explicit type of `Self`, or be specified with `self_type`"
            ),
        }
    };

    let vtable_constant_name = struct_name.to_string().to_uppercase() + "_VTBL";
    let vtable_constant_name = Ident::new(&vtable_constant_name, Span::call_site());

    let vtable_name = struct_name.to_string() + "VTable";
    let vtable_name = Ident::new(&vtable_name, Span::call_site());

    // we need to clone the function and move the body into a hidden one
    let mut hidden_fn = fn_input.clone();
    if hidden_fn.sig.ident != "new" {
        panic!("`#[new_with_vtable]` can only be used on functions called `new`")
    }
    hidden_fn.sig.ident = Ident::new("__new", Span::call_site());

    // add the `vtbl` parameter
    hidden_fn
        .sig
        .inputs
        .insert(0, FnArg::parse.parse2(quote! { vtbl: usize }).unwrap());

    // the last statement will be the return statement, grab either the struct def or ref
    let stct_statement = match hidden_fn
        .block
        .stmts
        .last()
        .expect("An empty `new` function is not allowed")
        .clone()
    {
        Stmt::Expr(Expr::Struct(_)) => match hidden_fn
            .block
            .stmts
            .last_mut()
            .expect("An empty `new` function is not allowed")
        {
            Stmt::Expr(Expr::Struct(stct)) => stct,
            _ => unreachable!(),
        },
        Stmt::Expr(Expr::Path(path)) => {
            // we need to track down where the path was instantiated
            match hidden_fn
                .block
                .stmts
                .iter_mut()
                .find(|stmt| match stmt {
                    Stmt::Local(Local {
                        pat: Pat::Ident(pat_ident),
                        init: Some(init),
                        ..
                    }) => {
                        path.path
                            .get_ident()
                            .map(|path_ident| path_ident == &pat_ident.ident)
                            .unwrap_or(false)
                            && match &*init.1 {
                                Expr::Struct(ExprStruct { path, .. }) => path
                                    .get_ident()
                                    .map(|type_ident| {
                                        type_ident == "Self" || type_ident == struct_name
                                    })
                                    .unwrap_or(false),
                                _ => false,
                            }
                    }
                    _ => false,
                })
                .expect("Definition for the return statement was not found")
            {
                Stmt::Local(Local {
                    init: Some(init), ..
                }) => match &mut *init.1 {
                    Expr::Struct(stct) => stct,
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            }
        }
        _ => panic!("The last statement in `new` must be an expression returning the new instance"),
    };

    // now the last scenario is can be a few possibilities:
    // - The base is initialized explicitly with a `new` function
    //  - Replace the call with `__new` and add the `vtbl` parameter to the function call
    // - The base is not initialized explicitly
    //  - We can assume it has a default implementation. Call `Base::__with_vtable(vtbl)`
    // - The struct is a base
    //  - We should set the field `vtbl`

    if let Some(base) = &attr.base {
        // see if we can find the initialization
        if stct_statement
            .fields
            .iter()
            .any(|field| match &field.member {
                Member::Named(ident) => ident == "base_with_vtbl",
                _ => panic!("#[new_with_vtable] can only be used on a struct with named fields!"),
            })
        {
            // TODO: figure out how to get rid of this duplicate here, and avoid borrowing twice
            let init = stct_statement
                .fields
                .iter_mut()
                .find(|field| match &field.member {
                    Member::Named(ident) => ident == "base_with_vtbl",
                    _ => {
                        panic!("#[new_with_vtable] can only be used on a struct with named fields!")
                    }
                })
                .unwrap();
            // ensure the initialization occurs with a function call
            match &mut init.expr {
                Expr::Call(call) => {
                    // update the name of the function
                    match &mut *call.func {
                        Expr::Path(path) if path.path.segments.last().unwrap().ident == "new" => {
                            *path.path.segments.last_mut().unwrap() = PathSegment::parse.parse2(quote!{ __new }).unwrap();
                        }
                        _ => panic!(
                            "Manually initializing `base_with_vtbl` requires a call to `Base::new`. If the \
                            intention is to default the value, simply leave it out"
                        ),
                    }

                    // add the vtable argument
                    call.args
                        .insert(0, Expr::parse.parse2(quote! { vtbl }).unwrap());
                }
                _ => panic!(
                    "Manually initializing `base_with_vtbl` requires a call to `Base::new`. If the \
                    intention is to default the value, simply leave it out"
                ),
            }
        } else {
            // they aren't explicitly instantiating. generate a call to `__with_vtbl`
            stct_statement.fields.insert(
                0,
                FieldValue::parse
                    .parse2(quote! { base_with_vtbl: #base::__with_vtbl(vtbl) })
                    .unwrap(),
            );
        }
    } else {
        // ensure they didn't manually set it, that's not supported
        if stct_statement
            .fields
            .iter()
            .any(|field| match &field.member {
                Member::Named(ident) => ident == "vtbl",
                _ => panic!("#[new_with_vtable] can only be used on a struct with named fields!"),
            })
        {
            panic!("Manually setting the `vtbl` field is unsupported!");
        }

        // add the field
        stct_statement
            .fields
            .insert(0, FieldValue::parse.parse2(quote! { vtbl }).unwrap());
    }

    let arg_names: Vec<&PatIdent> = fn_input
        .sig
        .inputs
        .iter()
        .map(|input| match input {
            FnArg::Typed(typ) => match &*typ.pat {
                Pat::Ident(ident) => ident,
                _ => panic!("All arguments in `new` must be named!"),
            },
            _ => panic!("`new` must not contain a `&self`!"),
        })
        .collect();

    // replace the `new` body with a call to the other
    fn_input.block.stmts =
        vec![Stmt::Expr(Expr::parse
        .parse2(quote! {
            Self::__new(&#vtable_constant_name as *const #vtable_name as usize, #(#arg_names),*)
        })
        .unwrap())];

    let res = quote! {
        #[allow(dead_code)]
        #fn_input
        #[doc(hidden)]
        #hidden_fn
    };
    res.into()
}
