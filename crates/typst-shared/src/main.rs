#![allow(dead_code)]
use crate::extended_info::ExtendedFileDescriptor;
use crate::memory_management::{Base16ByteArray, JavaResult, ThickBytePtr};
use std::any::type_name;
use typst::diag::FileResult;

pub mod java_world;
pub mod cache_cell;
pub mod query;
pub mod detached_eval;
pub mod compile;
pub mod exception;
pub mod memory_management;
pub mod extended_info;
pub mod fmt;
pub mod download;
pub mod terminal;

pub extern "C" fn main_nop() -> JavaResult<ExtendedFileDescriptor> {
    panic!()
}

pub extern "C" fn file_nop(_it: ThickBytePtr) -> JavaResult<FileResult<Base16ByteArray>> {
    panic!()
}

fn function_name<T>() -> &'static str {
    type_name::<T>()
}

fn main() {

}