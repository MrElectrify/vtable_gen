use itertools::Itertools;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::{
    Expr, ExprStruct, FnArg, ImplItem, ImplItemFn, ItemImpl, Member, parse_quote, Stmt, Type,
};

use crate::class::make_base_name;
use crate::class::vtable::{make_vtable_ident, make_vtable_static};
use crate::parse::{CppDef, ItemClass};
use crate::util::extract_ident;

/// Generates hooked versions of all implemented methods that construct instances.
pub fn gen_hooks(def: &CppDef) -> Option<ItemImpl> {
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
    imp.items = gen_impl_items(&def.class, imp.items);

    Some(imp)
}

/// Make a call to a constructor with a VTable passthrough.
pub fn make_ctor_call(ident: &Ident) -> Ident {
    format_ident!("_{ident}_with_vtable")
}

/// Generates all implementation items.
fn gen_impl_items(class: &ItemClass, items: Vec<ImplItem>) -> Vec<ImplItem> {
    items
        .into_iter()
        .flat_map(|item| match item {
            ImplItem::Fn(item_fn) => process_fn(class, item_fn)
                .into_iter()
                .map(ImplItem::Fn)
                .collect(),
            item => vec![item],
        })
        .collect()
}

/// Generates a stub of a function that calls the original implementation with the current
/// class's vtable.
fn gen_stub(class: &ItemClass, func: &ImplItemFn) -> ImplItemFn {
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
    let static_ident = make_vtable_static(&class.ident, class.generic_args());

    let output = quote! {
        #vis #unsafety #abi fn #ident(#args) #output {
            Self::#proxy_ident(#(#arg_names,)* &#static_ident)
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

/// Processes a function; if it needs a hook, 2 functions will be generated. Otherwise,
/// we panic. Standing functions should be kept outside the macro.
fn process_fn(class: &ItemClass, mut func: ImplItemFn) -> Vec<ImplItemFn> {
    // generate the stub function
    let stub_fn = gen_stub(class, &func);

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

    // add the vtable input parameter
    let generic_args = class.generic_args();
    let vtable_ty = make_vtable_ident(&class.ident);
    func.sig
        .inputs
        .push(parse_quote!(vfptr: &'static #vtable_ty #generic_args));

    // add the vtable instantiation
    let fn_ident = &func.sig.ident;
    for expr in instantiations {
        for base_ty in class
            .bases
            .bases
            .iter()
            .map(|(base, _)| extract_ident(base))
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
            call.args.push(parse_quote!(&vfptr.#base_ident));
        }

        if class.bases.is_empty() {
            expr.fields.insert(0, parse_quote!(vfptr))
        }
    }

    // rename this function and create another dummy
    func.sig.ident = make_ctor_call(&func.sig.ident);

    vec![func, stub_fn]
}
