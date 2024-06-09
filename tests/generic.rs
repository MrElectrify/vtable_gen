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
        N as usize + a as usize + b as usize
    }
}

#[test]
fn layout() {
    assert_eq!(
        std::mem::size_of::<Foo<2, u32>>(),
        std::mem::size_of::<usize>() * 2
    );
    assert_eq!(
        std::mem::size_of::<FooVTable<2, u32>>(),
        std::mem::size_of::<usize>() * 2
    );
}

#[test]
fn basic_foo() {
    let b = Foo::<2, u32>::new(2);

    // manually select the implementation
    assert_eq!(<Foo<2, u32> as FooVirtuals<2, u32>>::func(&b, 1, 2.0), 5);
    // call through the vtable
    assert_eq!(b.func(1, 2.0), 5);
}

#[test]
#[should_panic]
fn unimpl_method() {
    let b = Foo::<2, u32>::new(2);

    // ensure that unimplemented methods panic
    (unsafe { &*(b.vfptr as *const FooVTable<2, u32>) }.unimpl_0)()
}

#[test]
fn default() {
    let b = Foo::default();

    // manually select the implementation
    assert_eq!(<Foo<2, u32> as FooVirtuals<2, u32>>::func(&b, 1, 2.0), 5);
    // call through the vtable
    assert_eq!(b.func(1, 2.0), 5);
}
