use vtable_gen::cpp_class;

cpp_class! {
    #[derive(Debug, Default)]
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

cpp_class! {
    #[derive(Debug, Default)]
    struct Bar: Foo {
        b: f32

        virtual fn bar(&self) -> u32
    }

    impl Bar {
        fn new(a: f32, b: f32) -> Self {
            Self {
                base_foo: Foo::new(a),
                b
            }
        }
    }
}

impl FooVirtuals for Bar {
    extern "fastcall" fn func(this: &Foo, a: u32, b: f32) -> usize {
        this.a as usize + a as usize + b as usize + b as usize
    }
}

impl BarVirtuals for Bar {
    extern "C" fn bar(this: &Bar) -> u32 {
        (this.a + this.b) as u32
    }
}

#[test]
fn layout() {
    assert_eq!(std::mem::size_of::<Bar>(), std::mem::size_of::<usize>() * 3);
    assert_eq!(
        std::mem::size_of::<BarVTable>(),
        std::mem::size_of::<usize>() * 3
    );
}

#[test]
fn basic() {
    let b = Bar::new(2.5, 3.5);

    // manually select the implementation
    assert_eq!(<Foo as FooVirtuals>::func(&b, 1, 2.0), 5);
    assert_eq!(<Bar as FooVirtuals>::func(&b, 1, 2.0), 7);
    assert_eq!(<Bar as BarVirtuals>::bar(&b), 6);
    // call through the vtable
    assert_eq!(b.func(1, 2.0), 7);
    assert_eq!(b.bar(), 6);
}

#[test]
#[should_panic]
fn unimpl_method() {
    let b = Bar::new(2.5, 3.5);

    // ensure that unimplemented methods panic
    (unsafe { &*(b.vfptr as *const FooVTable) }.unimpl_0)()
}

#[test]
fn default() {
    let b = Bar::default();

    // manually select the implementation
    assert_eq!(<Foo as FooVirtuals>::func(&b, 1, 2.0), 3);
    assert_eq!(<Bar as FooVirtuals>::func(&b, 1, 2.0), 5);
    assert_eq!(<Bar as BarVirtuals>::bar(&b), 0);
    // call through the vtable
    assert_eq!(b.func(1, 2.0), 5);
    assert_eq!(b.bar(), 0);
}
