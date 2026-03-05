# VTable Generation with `vtable_gen`

A Rust procedural macro crate for interfacing with C++ classes that have virtual methods.

## Overview

The `vtable_gen` crate provides the `cpp_class!` macro for defining Rust types that mirror C++ classes with virtual tables. This enables:

- Calling C++ virtual methods from Rust
- Implementing C++ virtual methods in Rust
- Multiple inheritance support
- Type-safe vtable generation
- Cross-crate vtable definitions

## When to Use `cpp_class!`

**Use `cpp_class!` when**:
- The C++ type has virtual methods (has a vtable)
- The type inherits from a C++ class with virtual methods
- You need to implement virtual methods in Rust

**Do NOT use `cpp_class!` when**:
- The type is a plain struct without virtual methods
- The type is POD (Plain Old Data)
- No virtual method calls are needed

## Basic Pattern: Reading Virtual Methods

For types where you only need to call existing virtual methods (not implement them):

```rust
use vtable_gen::cpp_class;

cpp_class! {
    #[derive(Debug)]
    pub struct EventListener {
        // idx 0
        virtual unsafe fn on_event(&self, event: &Event),
        // idx 1
        virtual unsafe fn priority(&self) -> u16,
    }
}
```

**Key points**:
- No `#[gen_vtable]` attribute needed
- Virtual methods are declared but not implemented
- Used for calling into existing C++ code

## Implementing Virtual Methods

When you need to implement virtual methods in Rust (e.g., creating custom implementations):

### Step 1: Add `#[gen_vtable]` Attribute

```rust
cpp_class! {
    #[derive(Debug, Default)]
    #[gen_vtable]
    pub struct EventListener {
        virtual unsafe fn on_event(&self, event: &Event),
        virtual unsafe fn priority(&self) -> u16,
    }
}
```

This generates:
- `EventListenerVTable` struct
- `EventListenerVirtuals` trait
- `gen_event_listener_vtable!` macro (exported from the defining crate)

### Step 2: Import Required Types

**CRITICAL**: The `vtable_gen` crate works across crate boundaries and doesn't assume use paths. You MUST import:

1. The parent struct itself
2. `<ParentName>VTable` for each parent
3. `<ParentName>Virtuals` for each parent
4. `gen_<parent_name>_vtable` macro for each parent

```rust
// If EventListener is defined in another crate
use other_crate::{
    EventListener,
    EventListenerVTable,
    EventListenerVirtuals,
    gen_event_listener_vtable,
};
```

### Step 3: Implement the Virtuals Trait

```rust
impl EventListenerVirtuals for MyListener {
    extern "C" fn on_event(this: &EventListener, event: &Event) {
        // Your implementation
    }
    
    extern "C" fn priority(_: &EventListener) -> u16 {
        100
    }
}
```

### Step 4: Generate VTable with Macro

The `gen_*_vtable!` macro is automatically invoked when you use the type. The macro generates the vtable implementation for your concrete type.

## Multiple Inheritance

C++ classes can inherit from multiple base classes. The pattern extends naturally:

```rust
use vtable_gen::cpp_class;
use std::sync::{Arc, OnceLock};
use parking_lot::Mutex;

// Import all parent types, VTables, Virtuals, and generator macros
use base_crate::{
    Backend, BackendVTable, BackendVirtuals,
    SocketFactory, SocketFactoryVTable, SocketFactoryVirtuals,
    EventListener, EventListenerVTable, EventListenerVirtuals,
    gen_backend_vtable,
    gen_socket_factory_vtable,
    gen_event_listener_vtable,
};

cpp_class! {
    #[gen_vtable(no_unimpl)]
    pub struct CustomBackend: Backend, SocketFactory, EventListener {
        context: *mut Context,
        event_rx: OnceLock<Arc<Mutex<Receiver<Event>>>>,
    }
    
    impl CustomBackend {
        pub fn new(context: *mut Context) -> CustomBackend {
            Self {
                base_backend: Backend::default(),
                base_socket_factory: SocketFactory::default(),
                base_event_listener: EventListener::default(),
                context,
                event_rx: OnceLock::new(),
            }
        }
    }
}

impl BackendVirtuals for CustomBackend {
    extern "C" fn initialize(this: &mut Backend) {
        // Implementation
    }
    // ... other methods
}

impl SocketFactoryVirtuals for CustomBackend {
    extern "C" fn create_socket(_: &SocketFactory) -> *mut Socket {
        // Implementation
    }
}

impl EventListenerVirtuals for CustomBackend {
    extern "C" fn on_event(this: &EventListener, event: &Event) {
        // Implementation
    }
    extern "C" fn priority(_: &EventListener) -> u16 {
        100
    }
}
```

**Key points for multiple inheritance**:
- List all parent classes after the colon: `ChildClass: Parent1, Parent2, Parent3`
- Import `VTable` and `Virtuals` for EACH parent
- Import the `gen_*_vtable` macro for EACH parent
- Implement `*Virtuals` trait for EACH parent
- Constructor must initialize `base_<parent_name>` field for each parent

## The `#[gen_vtable]` Attribute

### `#[gen_vtable]`

Generates vtable with unimplemented default methods:

```rust
#[gen_vtable]
pub struct MyClass {
    virtual unsafe fn method(&self),
}
```

Generated methods will panic if called without implementation. Use when:
- You're only reading/calling existing virtual methods
- You want compile-time errors for missing implementations

### `#[gen_vtable(no_unimpl)]`

Generates vtable WITHOUT unimplemented defaults:

```rust
#[gen_vtable(no_unimpl)]
pub struct MyClass {
    virtual unsafe fn method(&self),
}
```

You MUST implement ALL virtual methods. Use when:
- Creating concrete implementations (backends, listeners, etc.)
- You will provide all method implementations
- You want to ensure all methods are implemented

## Constructor Pattern

When implementing a type with vtables, the constructor must initialize base class fields:

```rust
impl CustomBackend {
    pub fn new(context: *mut Context) -> CustomBackend {
        Self {
            // Initialize each parent class
            base_backend: Backend::default(),
            base_socket_factory: SocketFactory::default(),
            base_event_listener: EventListener::default(),
            
            // Initialize your own fields
            context,
            event_rx: OnceLock::new(),
        }
    }
}
```

**Field naming convention**:
- Parent class fields: `base_<parent_name_snake_case>`
- Example: `Backend` → `base_backend`
- Example: `SocketFactory` → `base_socket_factory`
- Example: `EventListener` → `base_event_listener`

## Down-Casting Pattern

When implementing virtual methods, you often receive a pointer to the base class but need access to your derived class. You'll need a down-casting mechanism:

```rust
// Example down-casting macro (you'll need to implement this)
macro_rules! down_cast_mut {
    ($ptr:expr, $from:ty, $to:ty, $field:ident) => {{
        let offset = memoffset::offset_of!($to, $field);
        ($ptr as *mut u8).sub(offset) as *mut $to
    }};
}

impl BackendVirtuals for CustomBackend {
    extern "C" fn update(this: &mut Backend, delta_time: f32) {
        // Cast from Backend to CustomBackend
        let this = unsafe {
            &mut *down_cast_mut!(
                this,
                Backend,
                CustomBackend,
                base_backend
            )
        };
        
        // Now you can access CustomBackend fields
        this.process_events();
    }
}
```

**Down-casting requirements**:
- Calculate offset from base class field to derived class start
- Subtract offset from base pointer to get derived pointer
- Common approach: use `memoffset::offset_of!` macro

## Complete Example: Custom Implementation

Here's a complete example showing all patterns together:

```rust
use vtable_gen::cpp_class;

// Assume these are defined in a base crate
use base_crate::{
    Backend, BackendVTable, BackendVirtuals,
    EventListener, EventListenerVTable, EventListenerVirtuals,
    gen_backend_vtable, gen_event_listener_vtable,
};

cpp_class! {
    #[gen_vtable(no_unimpl)]
    pub struct MyBackend: Backend, EventListener {
        custom_data: String,
    }
    
    impl MyBackend {
        pub fn new(context: *mut Context) -> MyBackend {
            Self {
                base_backend: Backend::new(context),
                base_event_listener: EventListener::default(),
                custom_data: String::new(),
            }
        }
    }
}

impl BackendVirtuals for MyBackend {
    extern "C" fn initialize(this: &mut Backend) -> bool {
        let this = unsafe {
            &mut *down_cast_mut!(this, Backend, MyBackend, base_backend)
        };
        
        // Initialize your backend
        this.custom_data = "initialized".to_string();
        true
    }
    
    extern "C" fn update(this: &mut Backend, delta_time: f32) {
        let this = unsafe {
            &mut *down_cast_mut!(this, Backend, MyBackend, base_backend)
        };
        
        // Update logic
        println!("Updating: {}", this.custom_data);
    }
    
    extern "C" fn shutdown(this: &mut Backend) {
        let this = unsafe {
            &mut *down_cast_mut!(this, Backend, MyBackend, base_backend)
        };
        
        this.custom_data.clear();
    }
}

impl EventListenerVirtuals for MyBackend {
    extern "C" fn on_event(this: &EventListener, event: &Event) {
        let this = unsafe {
            &mut *(down_cast_mut!(
                (this as *const EventListener as *mut EventListener),
                EventListener,
                MyBackend,
                base_event_listener
            ))
        };
        
        // Handle event
        println!("Event received: {}", this.custom_data);
    }
    
    extern "C" fn priority(_: &EventListener) -> u16 {
        100
    }
}
```

## Import Checklist

When working with vtables, ensure you import:

- [ ] Parent struct: `use wr_sdk::types::fb::ParentClass;`
- [ ] Parent VTable: `use wr_sdk::types::fb::ParentClassVTable;`
- [ ] Parent Virtuals: `use wr_sdk::types::fb::ParentClassVirtuals;`
- [ ] VTable generator: `use wr_sdk::gen_parent_class_vtable;`
- [ ] Repeat for EACH parent in multiple inheritance

## Common Patterns

### Extension Classes

Creating a new class that extends an existing one:

```rust
cpp_class! {
    #[gen_vtable]
    pub struct ExtendedBackend: Backend {
        virtual pub fn get_status(&self) -> Status,
        virtual pub fn get_info(&self, info: &mut String),
    }
    
    impl ExtendedBackend {
        pub fn new(context: *mut Context) -> ExtendedBackend {
            Self { base_backend: Backend::new(context) }
        }
    }
}
```

### Concrete Types with Derives

Types often use `#[gen_vtable(no_unimpl)]` with custom derives:

```rust
cpp_class! {
    #[derive(Debug, Clone)]
    #[gen_vtable(no_unimpl)]
    pub struct EventMessage: Message {
        pub timestamp: u64,
        pub data: Vec<u8>,
    }
}
```

## Troubleshooting

### "Cannot find `<Type>VTable` in scope"

You forgot to import the VTable type:
```rust
use wr_sdk::types::fb::ParentClassVTable;
```

### "Cannot find `<Type>Virtuals` in scope"

You forgot to import the Virtuals trait:
```rust
use wr_sdk::types::fb::ParentClassVirtuals;
```

### "Cannot find macro `gen_*_vtable`"

You forgot to import the generator macro:
```rust
use wr_sdk::gen_parent_class_vtable;
```

### Compilation errors about missing base fields

Ensure your constructor initializes all `base_<parent>` fields:
```rust
Self {
    base_parent_class: ParentClass::default(),
    // your fields...
}
```

## Advanced Topics

### Virtual Method Indices

Virtual methods are indexed starting from 0. You can skip indices using the `virtual(N)` syntax:

```rust
cpp_class! {
    pub struct MyClass {
        // idx 0
        virtual unsafe fn method_a(&self),
        // idx 1
        virtual unsafe fn method_b(&self),
        // idx 5 (skipping 2, 3, 4)
        virtual(5) unsafe fn method_c(&self),
    }
}
```

### Visibility Control

Virtual methods can have different visibility:

```rust
cpp_class! {
    pub struct MyClass {
        // Public virtual method
        virtual pub unsafe fn public_method(&self),
        // Private virtual method (default)
        virtual unsafe fn private_method(&self),
    }
}
```

### Const vs Mutable

Virtual methods can take `&self` or `&mut self`:

```rust
cpp_class! {
    pub struct MyClass {
        // Immutable receiver
        virtual unsafe fn read_only(&self) -> i32,
        // Mutable receiver
        virtual unsafe fn modify(&mut self, value: i32),
    }
}
```

## Limitations

- Single-level inheritance only (no grandparent access through intermediate parent)
- Virtual methods must use `extern "C"` ABI
- Down-casting requires manual offset calculation
- No automatic destructor generation

## Contributing

Contributions are welcome! Please ensure:
- All examples compile
- Documentation is clear and accurate
- Tests pass
- Code follows Rust conventions
