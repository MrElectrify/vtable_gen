use vtable_gen::cpp_class;

cpp_class! {
    #[derive(Default)]
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

cpp_class! {
    #[derive(Default)]
    #[gen_base(B)]
    struct D: C {
        d: u32,

        virtual fn d(&self) -> u32
    }

    impl D {
        fn new(a: u32, b: u32, c: u32, d: u32) -> Self {
            Self {
                base_c: C::new(a, b, c),
                d
            }
        }
    }
}

impl CVirtuals for D {
    extern "C" fn c(this: &C) -> u32 {
        this.a + this.c
    }
}

impl AVirtuals for D {
    extern "C" fn a(this: &A) -> u32 {
        this.a + 2
    }
}

impl BVirtuals for D {
    extern "C" fn b(this: &B) -> u32 {
        this.b + 2
    }
}

impl DVirtuals for D {
    extern "C" fn d(this: &D) -> u32 {
        this.d
    }
}

#[test]
fn layout() {
    assert_eq!(std::mem::size_of::<D>(), std::mem::size_of::<usize>() * 6);
    assert_eq!(
        std::mem::size_of::<DVTable>(),
        std::mem::size_of::<usize>() * 3
    );
}

#[test]
fn basic() {
    let c = D::new(1, 2, 3, 4);

    // manually select the implementation
    assert_eq!(<A as AVirtuals>::a(&c), 1);
    assert_eq!(<C as AVirtuals>::a(&c), 2);
    assert_eq!(<D as AVirtuals>::a(&c), 3);
    assert_eq!(<B as BVirtuals>::b(c.as_ref().as_ref()), 2);
    assert_eq!(<C as BVirtuals>::b(c.as_ref().as_ref()), 3);
    assert_eq!(<D as BVirtuals>::b(c.as_ref().as_ref()), 4);
    assert_eq!(<C as CVirtuals>::c(&c), 6);
    assert_eq!(<D as CVirtuals>::c(&c), 4);
    assert_eq!(<D as DVirtuals>::d(&c), 4);
    // call through the vtables
    assert_eq!(c.a(), 3);
    assert_eq!(<C as AsRef<B>>::as_ref(&c).b(), 4);
    assert_eq!(c.c(), 4);
    assert_eq!(c.d(), 4);
}

#[test]
fn default() {
    let c = D::default();

    // manually select the implementation
    assert_eq!(<A as AVirtuals>::a(&c), 0);
    assert_eq!(<C as AVirtuals>::a(&c), 1);
    assert_eq!(<D as AVirtuals>::a(&c), 2);
    assert_eq!(<B as BVirtuals>::b(c.as_ref().as_ref()), 0);
    assert_eq!(<C as BVirtuals>::b(c.as_ref().as_ref()), 1);
    assert_eq!(<D as BVirtuals>::b(c.as_ref().as_ref()), 2);
    assert_eq!(<C as CVirtuals>::c(&c), 0);
    assert_eq!(<D as CVirtuals>::c(&c), 0);
    assert_eq!(<D as DVirtuals>::d(&c), 0);
    // call through the vtables
    assert_eq!(c.a(), 2);
    assert_eq!(<C as AsRef<B>>::as_ref(&c).b(), 2);
    assert_eq!(c.c(), 0);
    assert_eq!(c.d(), 0);
}
