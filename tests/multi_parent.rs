use vtable_gen::cpp_class;

cpp_class! {
    #[derive(Default)]
    #[gen_vtable]
    struct A {
        a: u32,

        virtual fn a(&self) -> u32
    }

    impl A {
        fn new(a: u32) -> Self {
            Self { a }
        }
    }
}

impl AVirtuals for A {
    extern "C" fn a(this: &A) -> u32 {
        this.a
    }
}

cpp_class! {
    #[derive(Default)]
    #[gen_vtable]
    struct B {
        b: u32,

        virtual fn b(&self) -> u32
    }

    impl B {
        fn new(b: u32) -> Self {
            Self { b }
        }
    }
}

impl BVirtuals for B {
    extern "C" fn b(this: &B) -> u32 {
        this.b
    }
}

cpp_class! {
    #[derive(Default)]
    #[gen_vtable]
    struct C: A, B {
        c: u32,

        virtual fn c(&self) -> u32
    }

    impl C {
        fn new(a: u32, b: u32, c: u32) -> Self {
            Self {
                base_a: A::new(a),
                base_b: B::new(b),
                c
            }
        }
    }
}

impl AVirtuals for C {
    extern "C" fn a(this: &A) -> u32 {
        this.a + 1
    }
}

impl BVirtuals for C {
    extern "C" fn b(this: &B) -> u32 {
        this.b + 1
    }
}

impl CVirtuals for C {
    extern "C" fn c(this: &C) -> u32 {
        this.a + <C as AsRef<B>>::as_ref(this).b + this.c
    }
}

#[test]
fn layout() {
    assert_eq!(std::mem::size_of::<C>(), std::mem::size_of::<usize>() * 5);
    assert_eq!(
        std::mem::size_of::<CVTable>(),
        std::mem::size_of::<usize>() * 2
    );
}

#[test]
fn basic() {
    let c = C::new(1, 2, 3);

    // manually select the implementation
    assert_eq!(<A as AVirtuals>::a(&c), 1);
    assert_eq!(<C as AVirtuals>::a(&c), 2);
    assert_eq!(<C as BVirtuals>::b(c.as_ref()), 3);
    assert_eq!(<C as CVirtuals>::c(&c), 6);
    // call through the vtables
    assert_eq!(c.a(), 2);
    assert_eq!(<C as AsRef<B>>::as_ref(&c).b(), 3);
    assert_eq!(c.c(), 6);
}

#[test]
fn default() {
    let c = C::default();

    // manually select the implementation
    assert_eq!(<A as AVirtuals>::a(&c), 0);
    assert_eq!(<C as AVirtuals>::a(&c), 1);
    assert_eq!(<C as BVirtuals>::b(c.as_ref()), 1);
    assert_eq!(<C as CVirtuals>::c(&c), 0);
    // call through the vtables
    assert_eq!(c.a(), 1);
    assert_eq!(<C as AsRef<B>>::as_ref(&c).b(), 1);
    assert_eq!(c.c(), 0);
}
