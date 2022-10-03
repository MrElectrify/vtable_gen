# VTable Gen

This crate provides macros to generate C++-ABI VTables by defining the structure and vtable layout.
It also supports VTable inheritance and basic class inheritance.

# Examples

Check out `tests.rs`, which is pretty self-explanatory.

# Usage

## Base Structs
- Define a structure that contains virtual functions
- Define a structure for the VTable that exactly matches the name of the structure it belongs to,
followed by `VTable` exactly. Example:
```rs
struct Foo {}
struct FooVTable {}
```
- Mark both the VTable and structure with `#[gen_vtable]`. Any function pointers you include in
the VTable struct will require implementation in an automatically-generated `<name>Virtuals` trait.
Complete Example:
```rs
#[gen_vtable]
struct Foo {}
#[gen_vtable]
struct FooVTable {
    foo: extern "C" fn(this: &Foo) -> u32;
}
impl FooVirtuals for Foo {
    extern "C" fn foo(this: &Foo) -> u32 { todo!() }
}
```
## Derived Structs
- Define structures exactly as with base structures
- Include the attribute `base`. Example:
```rs
#[gen_vtable]
struct Foo {}
#[gen_vtable]
struct FooVTable {}

#[gen_vtable(base = "Foo")]
struct Bar {}
#[gen_vtable(base = "Foo")]
struct BarVTable {}
```

## Constructing Structs with VTables

Constructing structs with VTables is easy. If the struct is default-able, simply derive
`DefaultVTable` instead of `Default`. This will `impl Default`. If the struct isn't default-able,
define some function `fn new(/* args */) -> Self`. Mark the function with `new_with_vtable`,
supplying base structs if necessary as in `Derived Structs`. For the compiler to know the type,
you must either explicitly replace `Self` as the return type with the type itself, or specify
`self_type`. Here's a verbose example:

```rs
// ...
impl Bar {
    #[new_with_vtable(self_type = "Bar")]
    fn new(baz: u32) -> Self {
        Self { baz }
    }
}
```

which is also equivalent to

```rs
// ...
impl Bar {
    #[new_with_vtable]
    fn new(baz: u32) -> Bar {
        Self { baz }
    }
}
```

If there is a base struct that requires its `new` function to be called, you will have to also
explicitly initialize a `base_with_vtbl` member with the `new` constructor of the child type.
For example:

```rs
// ...
impl Bar {
    #[new_with_vtable(base = "Foo", self_type = "Bar")]
    fn new(baz: u32) -> Self {
        Self {
            base_with_vtable: Foo::new(123),
            baz
        }
    }
}
```

## Overriding Functions

Overriding functions is easy. Because all functions are defined in Traits, one can specify for the
compiler to not generate implementations for base struct `Virtuals` with the argument `no_base_trait_impl`
on the VTable (or both for symmetry :)).
Example:

```rs
// ...
#[gen_vtable(base = "Foo", no_base_trait_impl)]
struct BarVTable {}
// ...
impl FooVirtuals for Bar {
    extern "C" fn some_fn(this: &Foo) {
        // ...
    }
}
```

The only caveat is you will have to implement *all* base traits.

## Automatic Implementation

For an automatic implementation, in the case of some abstract struct for example, simply supply `unimpl`
as an argument to `gen_vtable`, and all methods will be implemented with `unimplemented!()`. Example:

```rs
// ...
#[gen_vtable(unimpl)]
struct Foo {}
#[gen_vtable(unimpl)]
struct FooVTable {}

// `FooVirtuals` is implemented for `Foo`
```

# Known Limitations
- `vtable_gen` currently does not support generic structs. This is a trivial addition, however, and
will likely be added in the future
