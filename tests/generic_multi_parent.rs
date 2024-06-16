use vtable_gen::cpp_class;

cpp_class! {
    #[derive(Default)]
    #[gen_vtable]
    struct A<const N: u32> {
        a: u32,

        virtual fn a(&self) -> u32
    }

    impl<const N: u32> A<N> {
        fn new(a: u32) -> Self {
            Self { a }
        }
    }
}

impl<const N: u32> AVirtuals<N> for A<N> {
    extern "C" fn a(this: &A<N>) -> u32 {
        this.a + N
    }
}

cpp_class! {
    #[derive(Default)]
    #[gen_vtable]
    struct B<T: Default, const N: u32> {
        b: T,

        virtual fn b(&self) -> u32
    }

    impl<T: Default, const N: u32> B<T, N> {
        fn new(b: T) -> Self {
            Self { b }
        }
    }
}

impl<T: Default, const N: u32> BVirtuals<T, N> for B<T, N> {
    extern "C" fn b(_this: &B<T, N>) -> u32 {
        N
    }
}

cpp_class! {
    #[derive(Default)]
    #[gen_vtable]
    struct C<const N: u32>: A<N>, B<u32, N> {
        c: u32,

        virtual fn c(&self) -> u32
    }

    impl<const N: u32> C<N> {
        fn new(a: u32, b: u32, c: u32) -> Self {
            Self {
                base_a: A::new(a),
                base_b: B::new(b),
                c
            }
        }
    }
}

impl<const N: u32> AVirtuals<N> for C<N> {
    extern "C" fn a(this: &A<N>) -> u32 {
        this.a + 1
    }
}

impl<const N: u32> BVirtuals<u32, N> for C<N> {
    extern "C" fn b(this: &B<u32, N>) -> u32 {
        this.b + 1
    }
}

impl<const N: u32> CVirtuals<N> for C<N> {
    extern "C" fn c(this: &C<N>) -> u32 {
        this.a + <C<N> as AsRef<B<u32, N>>>::as_ref(this).b + this.c
    }
}

#[test]
fn layout() {
    assert_eq!(
        std::mem::size_of::<C<11>>(),
        std::mem::size_of::<usize>() * 5
    );
    assert_eq!(
        std::mem::size_of::<CVTable<11>>(),
        std::mem::size_of::<usize>() * 2
    );
}

#[test]
fn basic() {
    let c = C::new(1, 2, 3);

    // manually select the implementation
    assert_eq!(<A<11> as AVirtuals<11>>::a(&c), 12);
    assert_eq!(<C<11> as AVirtuals<11>>::a(&c), 2);
    assert_eq!(<C<11> as BVirtuals<u32, 11>>::b(c.as_ref()), 3);
    assert_eq!(<C<11> as CVirtuals<11>>::c(&c), 6);
    // call through the vtables
    assert_eq!(c.a(), 2);
    assert_eq!(<C<11> as AsRef<B<u32, 11>>>::as_ref(&c).b(), 3);
    assert_eq!(c.c(), 6);
}

#[test]
fn default() {
    let c = C::default();

    // manually select the implementation
    assert_eq!(<A<11> as AVirtuals<11>>::a(&c), 11);
    assert_eq!(<C<11> as AVirtuals<11>>::a(&c), 1);
    assert_eq!(<C<11> as BVirtuals<u32, 11>>::b(c.as_ref()), 1);
    assert_eq!(<C<11> as CVirtuals<11>>::c(&c), 0);
    // call through the vtables
    assert_eq!(c.a(), 1);
    assert_eq!(<C<11> as AsRef<B<u32, 11>>>::as_ref(&c).b(), 1);
    assert_eq!(c.c(), 0);
}
