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

cpp_class! {
    #[derive(Debug, Default)]
    struct Baz: Bar {
        c: f32

        virtual fn baz(&self) -> u32
    }

    impl Baz {
        fn new(a: f32, b: f32, c: f32) -> Self {
            Self {
                base_bar: Bar::new(a, b),
                c
            }
        }
    }
}

impl FooVirtuals for Baz {
    extern "fastcall" fn func(this: &Foo, a: u32, b: f32) -> usize {
        this.a as usize + a as usize + b as usize + b as usize + b as usize
    }
}

impl BarVirtuals for Baz {
    extern "C" fn bar(this: &Bar) -> u32 {
        (this.a + this.b + this.b + 2.0) as u32
    }
}

impl BazVirtuals for Baz {
    extern "C" fn baz(this: &Baz) -> u32 {
        (this.a + this.b + this.c) as u32
    }
}

#[test]
fn layout() {
    assert_eq!(std::mem::size_of::<Baz>(), std::mem::size_of::<usize>() * 4);
    assert_eq!(
        std::mem::size_of::<BazVTable>(),
        std::mem::size_of::<usize>() * 4
    );
}

#[test]
fn basic() {
    let b = Baz::new(2.5, 3.5, 4.5);

    // manually select the implementation
    assert_eq!(<Foo as FooVirtuals>::func(&b, 1, 2.0), 5);
    assert_eq!(<Bar as FooVirtuals>::func(&b, 1, 2.0), 7);
    assert_eq!(<Baz as FooVirtuals>::func(&b, 1, 2.0), 9);
    assert_eq!(<Bar as BarVirtuals>::bar(&b), 6);
    assert_eq!(<Baz as BarVirtuals>::bar(&b), 11);
    assert_eq!(<Baz as BazVirtuals>::baz(&b), 10);
    // call through the vtable
    assert_eq!(b.func(1, 2.0), 9);
    assert_eq!(b.bar(), 11);
    assert_eq!(b.baz(), 10);
}

#[test]
#[should_panic]
fn unimpl_method() {
    let b = Baz::new(2.5, 3.5, 4.5);

    // ensure that unimplemented methods panic
    (unsafe { &*(b.vfptr as *const BazVTable) }
        .base_bar
        .base_foo
        .unimpl_0)()
}

#[test]
fn default() {
    let b = Baz::default();

    // manually select the implementation
    assert_eq!(<Foo as FooVirtuals>::func(&b, 1, 2.0), 3);
    assert_eq!(<Bar as FooVirtuals>::func(&b, 1, 2.0), 5);
    assert_eq!(<Baz as FooVirtuals>::func(&b, 1, 2.0), 7);
    assert_eq!(<Bar as BarVirtuals>::bar(&b), 0);
    assert_eq!(<Baz as BarVirtuals>::bar(&b), 2);
    assert_eq!(<Baz as BazVirtuals>::baz(&b), 0);
    // call through the vtable
    assert_eq!(b.func(1, 2.0), 7);
    assert_eq!(b.bar(), 2);
    assert_eq!(b.baz(), 0);
}
