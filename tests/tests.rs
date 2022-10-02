use vtable_gen::{gen_vtable, new_with_vtable, DefaultVTable};

#[derive(DefaultVTable)]
#[gen_vtable]
struct Foo {
    foo: u32,
}

#[gen_vtable]
struct FooVTable {
    foo: extern "C" fn(this: &Foo) -> u32,
}

impl FooVirtuals for Foo {
    extern "C" fn foo(_: &Foo) -> u32 {
        0
    }
}

#[derive(DefaultVTable)]
#[gen_vtable(base = "Foo")]
struct Bar {
    bar: u32,
}

impl Bar {
    #[new_with_vtable(base = "Foo")]
    fn new(bar: u32) -> Bar {
        Self { bar }
    }
}

#[gen_vtable(base = "Foo")]
struct BarVTable {
    bar: extern "C" fn(this: &Foo) -> u32,
}

impl BarVirtuals for Bar {
    extern "C" fn bar(_: &Foo) -> u32 {
        2
    }
}

#[gen_vtable(base = "Bar")]
struct Baz {
    baz: u32,
}

impl Baz {
    #[allow(clippy::disallowed_names)]
    #[new_with_vtable(base = "Bar")]
    fn new(baz: u32) -> Baz {
        Self {
            base_with_vtbl: Bar::new(23),
            baz,
        }
    }
}

#[gen_vtable(base = "Bar", no_base_trait_impl)]
struct BazVTable {
    baz: extern "C" fn(this: &Foo) -> u32,
}

impl FooVirtuals for Baz {
    extern "C" fn foo(_: &Foo) -> u32 {
        1
    }
}

impl BarVirtuals for Baz {
    extern "C" fn bar(_: &Foo) -> u32 {
        3
    }
}

impl BazVirtuals for Baz {
    extern "C" fn baz(_: &Foo) -> u32 {
        4
    }
}

#[test]
fn layout() {
    assert_eq!(std::mem::size_of::<Foo>(), std::mem::size_of::<usize>() * 2);
    assert_eq!(
        std::mem::size_of::<FooVTable>(),
        std::mem::size_of::<usize>()
    );
    assert_eq!(std::mem::size_of::<Bar>(), std::mem::size_of::<usize>() * 3);
    assert_eq!(
        std::mem::size_of::<BarVTable>(),
        std::mem::size_of::<usize>() * 2
    );
    assert_eq!(std::mem::size_of::<Baz>(), std::mem::size_of::<usize>() * 4);
    assert_eq!(
        std::mem::size_of::<BazVTable>(),
        std::mem::size_of::<usize>() * 3
    );
}

#[test]
fn basic_foo() {
    let f = Foo::default();

    assert_eq!(unsafe { *(f.vtbl as *const fn(&Foo) -> u32) }(&f), 0);
}

#[test]
fn basic_bar() {
    let b = Bar::default();

    assert_eq!(
        unsafe { *(b.base_with_vtbl.vtbl as *const fn(&Foo) -> u32) }(&b.base_with_vtbl),
        0
    );
    assert_eq!(
        unsafe { *(b.base_with_vtbl.vtbl as *const fn(&Foo) -> u32).add(1) }(&b.base_with_vtbl),
        2
    );
}

#[test]
fn basic_baz() {
    let b = Baz::new(5);

    assert_eq!(
        unsafe { *(b.base_with_vtbl.base_with_vtbl.vtbl as *const fn(&Foo) -> u32) }(
            &b.base_with_vtbl.base_with_vtbl
        ),
        1
    );
    assert_eq!(
        unsafe { *(b.base_with_vtbl.base_with_vtbl.vtbl as *const fn(&Foo) -> u32).add(1) }(
            &b.base_with_vtbl.base_with_vtbl
        ),
        3
    );
    assert_eq!(
        unsafe { *(b.base_with_vtbl.base_with_vtbl.vtbl as *const fn(&Foo) -> u32).add(2) }(
            &b.base_with_vtbl.base_with_vtbl
        ),
        4
    );
}
