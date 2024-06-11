use itertools::Itertools;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{
    Expr, ExprStruct, FnArg, ImplItem, ImplItemFn, ItemImpl, Member, parse_quote, Path,
    PathArguments, Stmt, Type,
};

use crate::class::make_base_name;
use crate::class::vtable::{make_vtable_ident, make_vtable_static};
use crate::parse::{CppDef, ItemClass};
use crate::util::{extract_ident, last_segment};

/// Generates hooked versions of all implemented methods that construct instances.
pub fn gen_hooks(def: &CppDef, additional_bases: &[Path]) -> Option<ItemImpl> {
    let mut imp = def.new_impl.clone()?;
    let ident = &def.class.ident;

    // make sure the impl is for us
    let Type::Path(ty) = &*imp.self_ty else {
        panic!("implementation of a non-type found")
    };
    assert_eq!(
        extract_ident(&ty.path),
        ident,
        "only implementations of the class type are allowed"
    );

    // generate new implementations (and pass through old ones, of course)
    imp.items = gen_impl_items(&def.class, imp.items, additional_bases);

    Some(imp)
}

/// Processes a function; if it needs a hook, 2 functions will be generated. Otherwise,
/// we panic. Standing functions should be kept outside the macro.
pub fn hook_fn(
    class: &ItemClass,
    mut func: ImplItemFn,
    additional_bases: &[Path],
) -> Vec<ImplItemFn> {
    // generate the stub function
    let stub_fn = gen_stub(class, &func, additional_bases);

    // look for all struct instantiations. this is a simple approach that only looks for
    // local declarations and raw expressions
    let instantiations: Vec<&mut ExprStruct> = func
        .block
        .stmts
        .iter_mut()
        .filter_map(|stmt| match stmt {
            Stmt::Local(local) => {
                if let Expr::Struct(stct) = &mut *local.init.as_mut()?.expr {
                    filter_instantiations(class, stct)
                } else {
                    None
                }
            }
            Stmt::Expr(Expr::Struct(stct), _) => filter_instantiations(class, stct),
            _ => None,
        })
        .collect();

    // if you see this, it's likely because we are very naive in our search for these expressions.
    // feel free to add your special case above and PR
    if instantiations.is_empty() {
        panic!("only impls that instantiate the target are allowed")
    }

    // the secondary base classes
    let generic_args = class.generic_args();
    let secondary_base_types = class
        .bases
        .paths()
        .skip(1)
        .chain(additional_bases)
        .collect_vec();
    let secondary_base_idents = secondary_base_types
        .iter()
        .cloned()
        .map(extract_ident)
        .collect_vec();
    let secondary_base_params = secondary_base_idents
        .iter()
        .cloned()
        .map(make_base_name)
        .collect_vec();

    // add the vtable input parameters
    let vtable_ty = make_vtable_ident(&class.ident);
    func.sig
        .inputs
        .push(parse_quote!(vfptr: &'static #vtable_ty #generic_args));
    for (base_ty, base_param) in secondary_base_types.iter().zip(&secondary_base_params) {
        let base_ident = extract_ident(base_ty);
        let vtable_ident = make_vtable_ident(base_ident);
        let vtable_generics = &last_segment(base_ty).arguments;
        func.sig
            .inputs
            .push(parse_quote!(#base_param: &'static #vtable_ident #vtable_generics))
    }

    // add the vtable instantiation
    let fn_ident = &func.sig.ident;
    for expr in instantiations {
        for (idx, base_ty) in class
            .bases
            .bases
            .iter()
            .map(|(base, _)| extract_ident(base))
            .enumerate()
        {
            // find the method that is called on the base type
            let base_ident = make_base_name(base_ty);
            let field_setter = expr
                .fields
                .iter_mut()
                .find(|field| match &field.member {
                    Member::Named(ident) => ident == &base_ident,
                    _ => false,
                })
                .unwrap_or_else(|| panic!("{base_ident} must be initialized in {fn_ident}"));

            // inject the vtable into the call
            let Expr::Call(call) = &mut field_setter.expr else {
                panic!("expected method call to instantiate {base_ident} in {fn_ident}")
            };
            let Expr::Path(fn_path) = &mut *call.func else {
                panic!(
                    "expected function call to instantiate {base_ident} in {fn_ident} to be a path"
                )
            };

            // replace the call itself
            let fn_segment = fn_path.path.segments.last_mut().unwrap_or_else(|| panic!("expected function call to instantiate {base_ident} in {fn_ident} to contain segments"));
            fn_segment.ident = make_ctor_call(&fn_segment.ident);

            // add the `vfptr` member
            if idx == 0 {
                call.args.push(parse_quote!(&vfptr.#base_ident));
                // add additional params
                // TODO: this will only work if only the primary base is multiply-inherited.
                // we need to pin additional bases to each base type
                for additional_base in additional_bases {
                    let last_segment = last_segment(additional_base);
                    let base_generics = if let PathArguments::AngleBracketed(def_generics) =
                        &last_segment.arguments
                    {
                        def_generics.clone()
                    } else {
                        parse_quote!(<>)
                    };
                    let vtable_static =
                        make_vtable_static(&class.ident, &last_segment.ident, &base_generics);
                    call.args.push(parse_quote!(&#vtable_static))
                }
            } else {
                let param = &secondary_base_params[idx - 1];
                call.args.push(parse_quote!(#param))
            }
        }

        if class.bases.is_empty() {
            expr.fields.insert(0, parse_quote!(vfptr))
        }
    }

    // rename this function and create another dummy
    func.sig.ident = make_ctor_call(&func.sig.ident);

    vec![func, stub_fn]
}

/// Make a call to a constructor with a VTable passthrough.
pub fn make_ctor_call(ident: &Ident) -> Ident {
    format_ident!("_{ident}_with_vtable")
}

/// Generates all implementation items.
fn gen_impl_items(
    class: &ItemClass,
    items: Vec<ImplItem>,
    additional_bases: &[Path],
) -> Vec<ImplItem> {
    items
        .into_iter()
        .flat_map(|item| match item {
            ImplItem::Fn(item_fn) => hook_fn(class, item_fn, additional_bases)
                .into_iter()
                .map(ImplItem::Fn)
                .collect(),
            item => vec![item],
        })
        .collect()
}

/// Generates a stub of a function that calls the original implementation with the current
/// class's vtable.
fn gen_stub(class: &ItemClass, func: &ImplItemFn, additional_bases: &[Path]) -> ImplItemFn {
    // create a proxy function
    let vis = &func.vis;
    let abi = &func.sig.abi;
    let unsafety = &func.sig.unsafety;
    let ident = &func.sig.ident;
    let args = &func.sig.inputs;
    let output = &func.sig.output;
    let arg_names = func
        .sig
        .inputs
        .iter()
        .map(|arg| match arg {
            FnArg::Typed(ty) => ty.pat.clone(),
            _ => unreachable!(),
        })
        .collect_vec();

    let proxy_ident = make_ctor_call(ident);
    let static_ident = make_vtable_static(&class.ident, &class.ident, &class.generic_args());

    // the secondary base classes
    let secondary_base_types = class
        .bases
        .paths()
        .skip(1)
        .chain(additional_bases)
        .collect_vec();
    let secondary_base_idents = secondary_base_types
        .iter()
        .map(|base_ident| {
            make_vtable_static(
                &class.ident,
                extract_ident(base_ident),
                &class.generic_args(),
            )
        })
        .collect_vec();

    let output = quote! {
        #vis #unsafety #abi fn #ident(#args) #output {
            Self::#proxy_ident(#(#arg_names,)* &#static_ident, #(&#secondary_base_idents),*)
        }
    };
    syn::parse(output.into()).expect("failed to generate stub")
}

/// Filters instantiations, only returning those that instantiate `self`.
fn filter_instantiations<'a>(
    class: &'a ItemClass,
    expr: &'a mut ExprStruct,
) -> Option<&'a mut ExprStruct> {
    if expr.path == parse_quote!(Self)
        || expr
            .path
            .segments
            .last()
            .expect("expected path segments")
            .ident
            == class.ident
    {
        Some(expr)
    } else {
        None
    }
}
