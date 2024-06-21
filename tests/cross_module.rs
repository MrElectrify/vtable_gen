//!
//! Copyright (C) Warsaw Revamped. Any unauthorized use, modification, or distribution of any portion of this file is prohibited. All rights reserved.
//!

use vtable_gen::cpp_class;

use crate::cross::A;
use crate::cross::AVTable;

mod cross;

cpp_class! {
    pub struct B: A {
        b: u32
    }
}
