use vtable_gen::cpp_class;

cpp_class! {
    #[derive(Debug, Default)]
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
    struct Bar<const N: u32, U: Default>: Foo<N, U> {
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

impl<const N: u32, T: Default> FooVirtuals<N, T> for Bar<N, T> {
    extern "fastcall" fn func(_this: &Foo<N, T>, a: u32, b: f32) -> usize {
        a as usize + b as usize + N as usize
    }
}

impl<const N: u32, U: Default> BarVirtuals<N, U> for Bar<N, U> {
    extern "C" fn bar(this: &Bar<N, U>) -> &U {
        &this.a
    }
}

cpp_class! {
    #[derive(Debug, Default)]
    struct Baz<T: Default>: Bar<19, T> {
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

impl<T: Default> BarVirtuals<19, T> for Baz<T> {
    extern "C" fn bar(this: &Bar<19, T>) -> &T {
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
    assert_eq!(<Bar<19, u32> as FooVirtuals<19, u32>>::func(&b, 1, 2.0), 22);
    assert_eq!(<Baz<u32> as FooVirtuals<19, u32>>::func(&b, 1, 2.0), 5);
    assert_eq!(<Bar<19, u32> as BarVirtuals<19, u32>>::bar(&b), &1);
    assert_eq!(<Baz<u32> as BarVirtuals<19, u32>>::bar(&b), &2);
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
    (unsafe { &*(b.vfptr as *const BazVTable<u32>) }
        .base_bar
        .base_foo
        .unimpl_0)()
}

#[test]
fn default() {
    let b = Baz::default();

    // manually select the implementation
    assert_eq!(<Foo<19, u32> as FooVirtuals<19, u32>>::func(&b, 1, 2.0), 3);
    assert_eq!(<Bar<19, u32> as FooVirtuals<19, u32>>::func(&b, 1, 2.0), 22);
    assert_eq!(<Baz<u32> as FooVirtuals<19, u32>>::func(&b, 1, 2.0), 5);
    assert_eq!(<Bar<19, u32> as BarVirtuals<19, u32>>::bar(&b), &0);
    assert_eq!(<Baz<u32> as BarVirtuals<19, u32>>::bar(&b), &0);
    assert_eq!(<Baz<u32> as BazVirtuals<u32>>::baz(&b), 123);
    // call through the vtable
    assert_eq!(b.func(1, 2.0), 5);
    assert_eq!(b.bar(), &0);
    assert_eq!(b.baz(), 123);
}
