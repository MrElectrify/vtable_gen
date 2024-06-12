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
    #[no_impl]
    struct D: C {
        d: u32,

        virtual fn d(&self) -> u32
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

extern "C" fn fn_d(this: &D) -> u32 {
    this.d + 4
}

const D_VTBL: DVTable = DVTable {
    base_c: C::VTBL_FOR_C,
    d: fn_d,
};

#[test]
fn basic() {
    let d = D {
        base_c: C::_new_with_vtable(1, 2, 3, &D_VTBL.base_c, &C::VTBL_FOR_B),
        d: 12,
    };

    assert_eq!(d.a(), 2);
    assert_eq!(<C as AsRef<B>>::as_ref(&d).b(), 3);
    assert_eq!(d.c(), 6);
    assert_eq!(d.d(), 16)
}
