use vtable_gen::cpp_class;

cpp_class! {
    #[derive(Debug, Default)]
    struct FooImpl {
        virtual(1) fn foo(&self) -> u32,
    }
}

impl FooImplVirtuals for FooImpl {
    extern "C" fn foo(this: &FooImpl) -> u32 {
        this.foo() + 31
    }
}

// currently, we don't support impls. you can do them manually if needed
cpp_class! {
    #[impl_generic_base([T = FooImpl])]
    #[derive(Debug, Default)]
    struct Foo<T>: T {
        a: u32

        virtual(1) extern "fastcall" fn foo2(&self, a: u32, b: f32) -> usize,
    }
}

impl FooImplVirtuals for Foo_FooImpl {
    extern "C" fn foo(_this: &FooImpl) -> u32 {
        13
    }
}

impl Foo_FooImplVirtuals for Foo_FooImpl {
    extern "fastcall" fn foo2(this: &Foo_FooImpl, a: u32, b: f32) -> usize {
        this.base_foo_impl.foo() as usize + a as usize + b as usize
    }
}

cpp_class! {
    #[impl_generic_base([T = FooImpl])]
    struct Bar<T>: Foo<T> {
        virtual extern "fastcall" fn bar(&self) -> u32,
    }
}

impl FooImplVirtuals for Bar_FooImpl {
    extern "C" fn foo(_this: &FooImpl) -> u32 {
        17
    }
}

impl Foo_FooImplVirtuals for Bar_FooImpl {
    extern "fastcall" fn foo2(this: &Foo_FooImpl, a: u32, b: f32) -> usize {
        this.a as usize + a as usize - b as usize
    }
}

impl Bar_FooImplVirtuals for Bar_FooImpl {
    extern "fastcall" fn bar(_this: &Bar_FooImpl) -> u32 {
        19
    }
}

#[test]
fn layout() {
    assert_eq!(
        std::mem::size_of::<Foo_FooImpl>(),
        std::mem::size_of::<usize>() * 2
    );
    assert_eq!(
        std::mem::size_of::<Foo_FooImplVTable>(),
        std::mem::size_of::<usize>() * 2
    );
}

#[test]
#[should_panic]
fn unimpl_method() {
    let b = Foo_FooImpl::default();

    // ensure that unimplemented methods panic
    (unsafe { &*(b.base_foo_impl.vfptr as *const FooImplVTable) }.unimpl_0)()
}

#[test]
fn default() {
    let b = Foo_FooImpl::default();

    // manually select the implementation
    assert_eq!(<Foo_FooImpl as FooImplVirtuals>::foo(&b), 13);
    assert_eq!(<Foo_FooImpl as Foo_FooImplVirtuals>::foo2(&b, 1, 2.0), 16);
    // call through the vtable
    assert_eq!(b.foo(), 13);
    assert_eq!(b.foo2(1, 2.0), 16);
}
