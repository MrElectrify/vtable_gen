use vtable_gen::cpp_class;

cpp_class! {
    #[derive(Debug, Default)]
    #[gen_vtable]
    struct Foo<const N: u32, T: Default> {
        a: T,

        virtual(1) extern "fastcall" fn func(&self, a: u32, b: f32) -> usize,
    }

    impl<const N: u32, T: Default> Foo<N, T> {
        /// Creates a new `Foo`.
        fn new(a: T) -> Self {
            Self { a }
        }
    }
}

impl<const N: u32, T: Default> FooVirtuals<N, T> for Foo<N, T> {
    extern "fastcall" fn func(_this: &Foo<N, T>, a: u32, b: f32) -> usize {
        a as usize + b as usize
    }
}

cpp_class! {
    #[derive(Debug, Default)]
    #[gen_vtable]
    struct Bar<U: Default, const O: u32>: Foo<O, U> {
        b: U

        virtual fn bar(&self) -> &U
    }

    impl<U: Default, const O: u32> Bar<U, O> {
        fn new(a: U, b: U) -> Self {
            Self {
                base_foo: Foo::new(a),
                b
            }
        }
    }
}

impl<U: Default, const O: u32> FooVirtuals<O, U> for Bar<U, O> {
    extern "fastcall" fn func(_this: &Foo<O, U>, a: u32, b: f32) -> usize {
        a as usize + b as usize + O as usize
    }
}

impl<U: Default, const O: u32> BarVirtuals<U, O> for Bar<U, O> {
    extern "C" fn bar(this: &Bar<U, O>) -> &U {
        &this.a
    }
}

cpp_class! {
    #[derive(Debug, Default)]
    #[gen_vtable]
    struct Baz<T: Default>: Bar<T, 19> {
        c: T

        virtual fn baz(&self) -> u32
    }

    impl<T: Default> Baz<T> {
        fn new(a: T, b: T, c: T) -> Self {
            Self {
                base_bar: Bar::new(a, b),
                c
            }
        }
    }
}

impl<T: Default> FooVirtuals<19, T> for Baz<T> {
    extern "fastcall" fn func(_this: &Foo<19, T>, a: u32, b: f32) -> usize {
        a as usize + b as usize + 2
    }
}

impl<T: Default> BarVirtuals<T, 19> for Baz<T> {
    extern "C" fn bar(this: &Bar<T, 19>) -> &T {
        &this.b
    }
}

impl<T: Default> BazVirtuals<T> for Baz<T> {
    extern "C" fn baz(_this: &Baz<T>) -> u32 {
        123
    }
}

#[test]
fn layout() {
    assert_eq!(
        std::mem::size_of::<Baz<u32>>(),
        std::mem::size_of::<usize>() * 4
    );
    assert_eq!(
        std::mem::size_of::<BazVTable<u32>>(),
        std::mem::size_of::<usize>() * 4
    );
}

#[test]
fn basic() {
    let b = Baz::<u32>::new(1, 2, 3);

    // manually select the implementation
    assert_eq!(<Foo<19, u32> as FooVirtuals<19, u32>>::func(&b, 1, 2.0), 3);
    assert_eq!(<Bar<u32, 19> as FooVirtuals<19, u32>>::func(&b, 1, 2.0), 22);
    assert_eq!(<Baz<u32> as FooVirtuals<19, u32>>::func(&b, 1, 2.0), 5);
    assert_eq!(<Bar<u32, 19> as BarVirtuals<u32, 19>>::bar(&b), &1);
    assert_eq!(<Baz<u32> as BarVirtuals<u32, 19>>::bar(&b), &2);
    assert_eq!(<Baz<u32> as BazVirtuals<u32>>::baz(&b), 123);
    // call through the vtable
    assert_eq!(b.func(1, 2.0), 5);
    assert_eq!(b.bar(), &2);
    assert_eq!(b.baz(), 123);
}

#[test]
#[should_panic]
fn unimpl_method() {
    let b = Baz::<u32>::new(1, 2, 3);

    // ensure that unimplemented methods panic
    (unsafe { &*(b.vfptr as *const FooVTable<19, u32>) }.unimpl_0)()
}

#[test]
fn default() {
    let b = Baz::default();

    // manually select the implementation
    assert_eq!(<Foo<19, u32> as FooVirtuals<19, u32>>::func(&b, 1, 2.0), 3);
    assert_eq!(<Bar<u32, 19> as FooVirtuals<19, u32>>::func(&b, 1, 2.0), 22);
    assert_eq!(<Baz<u32> as FooVirtuals<19, u32>>::func(&b, 1, 2.0), 5);
    assert_eq!(<Bar<u32, 19> as BarVirtuals<u32, 19>>::bar(&b), &0);
    assert_eq!(<Baz<u32> as BarVirtuals<u32, 19>>::bar(&b), &0);
    assert_eq!(<Baz<u32> as BazVirtuals<u32>>::baz(&b), 123);
    // call through the vtable
    assert_eq!(b.func(1, 2.0), 5);
    assert_eq!(b.bar(), &0);
    assert_eq!(b.baz(), 123);
}
