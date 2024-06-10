use vtable_gen::cpp_class;

cpp_class! {
    #[derive(Debug, Default)]
    struct Foo<T: Default> {
        a: T,

        virtual(1) extern "fastcall" fn func(&self, a: u32, b: f32) -> usize,
    }

    impl<T: Default> Foo<T> {
        /// Creates a new `Foo`.
        fn new(a: T) -> Self {
            Self { a }
        }
    }
}

impl<T: Default> FooVirtuals<T> for Foo<T> {
    extern "fastcall" fn func(_this: &Foo<T>, a: u32, b: f32) -> usize {
        a as usize + b as usize
    }
}

cpp_class! {
    #[derive(Debug, Default)]
    struct Bar<const N: u32, U: Default>: Foo<U> {
        b: U

        virtual fn bar(&self) -> &U
    }

    impl<const N: u32, U: Default> Bar<N, U> {
        fn new(a: U, b: U) -> Self {
            Self {
                base_foo: Foo::new(a),
                b
            }
        }
    }
}

impl<const N: u32, T: Default> FooVirtuals<T> for Bar<N, T> {
    extern "fastcall" fn func(_this: &Foo<T>, a: u32, b: f32) -> usize {
        a as usize + b as usize + N as usize
    }
}

impl<const N: u32, U: Default> BarVirtuals<N, U> for Bar<N, U> {
    extern "C" fn bar(this: &Bar<N, U>) -> &U {
        &this.a
    }
}

#[test]
fn layout() {
    assert_eq!(
        std::mem::size_of::<Bar<23, u32>>(),
        std::mem::size_of::<usize>() * 3
    );
    assert_eq!(
        std::mem::size_of::<BarVTable<23, u32>>(),
        std::mem::size_of::<usize>() * 3
    );
}

#[test]
fn basic() {
    let b = Bar::<23, u32>::new(2, 3);

    // manually select the implementation
    assert_eq!(<Foo<u32> as FooVirtuals<u32>>::func(&b, 1, 2.0), 3);
    assert_eq!(<Bar<23, u32> as FooVirtuals<u32>>::func(&b, 1, 2.0), 26);
    assert_eq!(<Bar<23, u32> as BarVirtuals<23, u32>>::bar(&b), &2);
    // call through the vtable
    assert_eq!(b.func(1, 2.0), 26);
    assert_eq!(b.bar(), &2);
}

#[test]
#[should_panic]
fn unimpl_method() {
    let b = Bar::<23, u32>::new(2, 3);

    // ensure that unimplemented methods panic
    (unsafe { &*(b.vfptr as *const BarVTable<23, u32>) }
        .base_foo
        .unimpl_0)()
}

#[test]
fn default() {
    let b = Bar::default();

    // manually select the implementation
    assert_eq!(<Foo<u32> as FooVirtuals<u32>>::func(&b, 1, 2.0), 3);
    assert_eq!(<Bar<23, u32> as FooVirtuals<u32>>::func(&b, 1, 2.0), 26);
    assert_eq!(<Bar<23, u32> as BarVirtuals<23, u32>>::bar(&b), &0);
    // call through the vtable
    assert_eq!(b.func(1, 2.0), 26);
    assert_eq!(b.bar(), &0);
}
