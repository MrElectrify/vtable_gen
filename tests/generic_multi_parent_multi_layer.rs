use vtable_gen::cpp_class;

cpp_class! {
    #[derive(Default)]
    #[gen_vtable(no_unimpl)]
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
    #[gen_vtable(no_unimpl)]
    struct B<T: Default, const O: u32> {
        b: T,

        virtual fn b(&self) -> u32
    }

    impl<T: Default, const O: u32> B<T, O> {
        fn new(b: T) -> Self {
            Self { b }
        }
    }
}

impl<T: Default, const O: u32> BVirtuals<T, O> for B<T, O> {
    extern "C" fn b(_this: &B<T, O>) -> u32 {
        O
    }
}

cpp_class! {
    #[derive(Default)]
    #[gen_vtable(no_unimpl)]
    struct C<const P: u32, U: Default>: A<P>, B<U, P> {
        c: u32,

        virtual fn c(&self) -> u32
    }

    impl<const P: u32, U: Default> C<P, U> {
        fn new(a: u32, b: U, c: u32) -> Self {
            Self {
                base_a: A::new(a),
                base_b: B::new(b),
                c
            }
        }
    }
}

impl<const P: u32, U: Default> AVirtuals<P> for C<P, U> {
    extern "C" fn a(this: &A<P>) -> u32 {
        this.a + 1
    }
}

impl<const P: u32, U: Default> BVirtuals<U, P> for C<P, U> {
    extern "C" fn b(_this: &B<U, P>) -> u32 {
        P + 1
    }
}

impl<const P: u32, U: Default> CVirtuals<P, U> for C<P, U> {
    extern "C" fn c(this: &C<P, U>) -> u32 {
        this.a + this.c
    }
}

cpp_class! {
    #[derive(Default)]
    #[gen_base(C<Q, u32> = [B<u32, Q>])]
    #[gen_vtable(no_unimpl)]
    struct D<const Q: u32>: C<Q, u32> {
        d: u32,

        virtual fn d(&self) -> u32
    }

    impl<const Q: u32> D<Q> {
        fn new(a: u32, b: u32, c: u32, d: u32) -> Self {
            Self {
                base_c: C::new(a, b, c),
                d
            }
        }
    }
}

impl<const Q: u32> CVirtuals<Q, u32> for D<Q> {
    extern "C" fn c(this: &C<Q, u32>) -> u32 {
        this.a + this.c + Q
    }
}

impl<const Q: u32> AVirtuals<Q> for D<Q> {
    extern "C" fn a(this: &A<Q>) -> u32 {
        this.a + 2
    }
}

impl<const Q: u32> BVirtuals<u32, Q> for D<Q> {
    extern "C" fn b(this: &B<u32, Q>) -> u32 {
        this.b + 2
    }
}

impl<const Q: u32> DVirtuals<Q> for D<Q> {
    extern "C" fn d(this: &D<Q>) -> u32 {
        this.d
    }
}

#[test]
fn layout() {
    assert_eq!(
        std::mem::size_of::<D<23>>(),
        std::mem::size_of::<usize>() * 6
    );
    assert_eq!(
        std::mem::size_of::<DVTable<23>>(),
        std::mem::size_of::<usize>() * 3
    );
}

#[test]
fn basic() {
    let c = D::new(1, 2, 3, 4);

    // manually select the implementation
    assert_eq!(<A<23> as AVirtuals<23>>::a(&c), 24);
    assert_eq!(<C<23, u32> as AVirtuals<23>>::a(&c), 2);
    assert_eq!(<D<23> as AVirtuals<23>>::a(&c), 3);
    assert_eq!(
        <B<u32, 23> as BVirtuals<u32, 23>>::b(c.as_ref().as_ref()),
        23
    );
    assert_eq!(
        <C<23, u32> as BVirtuals<u32, 23>>::b(c.as_ref().as_ref()),
        24
    );
    assert_eq!(<D<23> as BVirtuals<u32, 23>>::b(c.as_ref().as_ref()), 4);
    assert_eq!(<C<23, u32> as CVirtuals<23, u32>>::c(&c), 4);
    assert_eq!(<D<23> as CVirtuals<23, u32>>::c(&c), 27);
    assert_eq!(<D<23> as DVirtuals<23>>::d(&c), 4);
    // call through the vtables
    assert_eq!(c.a(), 3);
    assert_eq!(<C<23, u32> as AsRef<B<u32, 23>>>::as_ref(&c).b(), 4);
    assert_eq!(c.c(), 27);
    assert_eq!(c.d(), 4);
}

#[test]
fn default() {
    let c = D::default();

    // manually select the implementation
    assert_eq!(<A<23> as AVirtuals<23>>::a(&c), 23);
    assert_eq!(<C<23, u32> as AVirtuals<23>>::a(&c), 1);
    assert_eq!(<D<23> as AVirtuals<23>>::a(&c), 2);
    assert_eq!(
        <B<u32, 23> as BVirtuals<u32, 23>>::b(c.as_ref().as_ref()),
        23
    );
    assert_eq!(
        <C<23, u32> as BVirtuals<u32, 23>>::b(c.as_ref().as_ref()),
        24
    );
    assert_eq!(<D<23> as BVirtuals<u32, 23>>::b(c.as_ref().as_ref()), 2);
    assert_eq!(<C<23, u32> as CVirtuals<23, u32>>::c(&c), 0);
    assert_eq!(<D<23> as CVirtuals<23, u32>>::c(&c), 23);
    assert_eq!(<D<23> as DVirtuals<23>>::d(&c), 0);
    // call through the vtables
    assert_eq!(c.a(), 2);
    assert_eq!(<C<23, u32> as AsRef<B<u32, 23>>>::as_ref(&c).b(), 2);
    assert_eq!(c.c(), 23);
    assert_eq!(c.d(), 0);
}
