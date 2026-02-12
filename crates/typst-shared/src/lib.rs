// Added by LDemetrios

use std::backtrace::Backtrace;
use std::panic;

pub mod export;
pub mod terminal;
pub mod syntax;
pub mod serial;
pub mod world;

pub use export::query::*;
pub use world::time::Now;

pub use self::{
    export::*,
    serial::*,
    syntax::*,
    terminal::*,
};

#[unsafe(no_mangle)]
pub extern "C" fn boot_driver() {
    panic::set_hook(Box::new(|_| {
        println!("{:?}", Backtrace::force_capture());
        unsafe { throw_exception() };
    }));
}

unsafe extern "C" {
    fn throw_exception();
}
