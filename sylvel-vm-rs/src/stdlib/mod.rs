// stdlib/mod.rs

pub mod io;
pub mod math;
pub mod string;
pub mod array;
pub mod sys;

use crate::vm::Vm;

pub fn register_all(vm: &mut Vm) {
    io::register(vm);
    math::register(vm);
    string::register(vm);
    array::register(vm);
    sys::register(vm);
}
