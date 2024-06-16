use vtable_gen::cpp_class;

cpp_class! {
    #[derive(Debug, Default)]
    #[gen_vtable]
    struct Foo {
        a: f32,

        virtual(1) extern "fastcall" fn func(&self, a: u32, b: f32) -> usize,
    }

    impl Foo {
        /// Creates a new `Foo`.
        fn new(a: f32) -> Self {
            Self { a }
        }
    }
}

impl FooVirtuals for Foo {
    extern "fastcall" fn func(this: &Foo, a: u32, b: f32) -> usize {
        this.a as usize + a as usize + b as usize
    }
}

#[test]
fn layout() {
    assert_eq!(std::mem::size_of::<Foo>(), std::mem::size_of::<usize>() * 2);
    assert_eq!(
        std::mem::size_of::<FooVTable>(),
        std::mem::size_of::<usize>() * 2
    );
}

#[test]
fn basic_foo() {
    let b = Foo::new(2.5);

    // manually select the implementation
    assert_eq!(<Foo as FooVirtuals>::func(&b, 1, 2.0), 5);
    // call through the vtable
    assert_eq!(b.func(1, 2.0), 5);
}

#[test]
#[should_panic]
fn unimpl_method() {
    let b = Foo::new(2.5);

    // ensure that unimplemented methods panic
    (unsafe { &*(b.vfptr as *const FooVTable) }.unimpl_0)()
}

#[test]
fn default() {
    let b = Foo::default();

    // manually select the implementation
    assert_eq!(<Foo as FooVirtuals>::func(&b, 1, 2.0), 3);
    // call through the vtable
    assert_eq!(b.func(1, 2.0), 3);
}
