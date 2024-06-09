use syn::{Expr, ExprStruct, ImplItem, ImplItemFn, ItemImpl, parse_quote, Stmt, Type};

use crate::class::vtable::make_vtable_static;
use crate::parse::{CppDef, ItemClass};

/// Generates hooked versions of all implemented methods that construct instances.
pub fn gen_hooks(def: &CppDef) -> Option<ItemImpl> {
    let mut imp = def.new_impl.clone()?;

    // make sure the impl is for us
    let Type::Path(ty) = &*imp.self_ty else {
        panic!("implementation of a non-type found")
    };
    assert_eq!(
        ty.path.get_ident(),
        Some(&def.class.ident),
        "only implementations of the class type are allowed"
    );

    // generate new implementations (and pass through old ones, of course)
    imp.items = gen_impl_items(&def.class, imp.items);

    Some(imp)
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
fn process_fn(class: &ItemClass, mut item: ImplItemFn) -> Vec<ImplItemFn> {
    // look for all struct instantiations. this is a simple approach that only looks for
    // local declarations and raw expressions
    let instantiations: Vec<&mut ExprStruct> = item
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

    // add the vtable instantiation
    let vtable_static_ident = make_vtable_static(&class.ident);
    for expr in instantiations {
        expr.fields.insert(
            0,
            parse_quote! { vfptr: &#vtable_static_ident as *const _ as usize },
        )
    }

    vec![item]
}
