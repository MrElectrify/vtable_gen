use vtable_gen::cpp_class;

cpp_class! {
    #[derive(Default)]
    #[gen_vtable(no_unimpl)]
    pub struct A {
        pub a: u32,

        virtual fn a(&self) -> u32
    }
}

impl AVirtuals for A {
    extern "C" fn a(_this: &A) -> u32 {
        123
    }
}
