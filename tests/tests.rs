use vtable_gen::cpp_class;

cpp_class! {
    #[derive(Debug)]
    struct Foo {
        a: f32,

        virtual(1) extern "fastcall" fn func(&self, a: u32, b: f32) -> usize,
    }
}

impl FooVirtuals for Foo {
    extern "fastcall" fn func(this: &Foo, a: u32, b: f32) -> usize {
        this.a as usize + a as usize + b as usize
    }
}

#[test]
fn layout() {
    assert_eq!(std::mem::size_of::<Foo>(), std::mem::size_of::<usize>() * 2);
    assert_eq!(
        std::mem::size_of::<FooVTable>(),
        std::mem::size_of::<usize>() * 2
    );
}

#[test]
fn basic_foo() {
    let b = Foo {
        vfptr: &FOO_VTBL as *const _ as usize,
        a: 2.5,
    };

    assert_eq!(<Foo as FooVirtuals>::func(&b, 1, 2.0), 5);
    assert_eq!(b.func(1, 2.0), 5);
}

#[test]
#[should_panic]
fn unimpl_method() {
    let b = Foo {
        vfptr: &FOO_VTBL as *const _ as usize,
        a: 2.5,
    };

    (unsafe { &*(b.vfptr as *const FooVTable) }.unimpl_0)()
}
