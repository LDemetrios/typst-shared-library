// Added by LDemetrios

use crate::export::raw_string::RawString;
use crate::flattened_tree;
use std::marker::PhantomData;
use std::mem;
use typst::syntax::{parse, parse_code, parse_math};

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CVec<T> {
    pub ptr: i64,
    pub len: i64,
    pub cap: i64,
    pub phantom_data: PhantomData<T>
}

impl<T> From<Vec<T>> for CVec<T> {
    fn from(value: Vec<T>) -> Self {
        let res = CVec {
            ptr: value.as_ptr() as i64,
            len: value.len() as i64,
            cap: value.capacity() as i64,
            phantom_data: PhantomData::default(),
        };
        mem::forget(value);
        res
    }
}

impl<T> From<CVec<T>> for Vec<T> {
    fn from(value: CVec<T>) -> Self {
        unsafe { Vec::from_raw_parts(value.ptr as *mut T, value.len as usize, value.cap as usize) }
    }
}

#[repr(C)]
pub struct CFlattenedSyntaxTree {
    pub marks: CVec<i64>,
    pub errors: CVec<u8>,
    pub errors_starts: CVec<i32>,
}

#[unsafe(no_mangle)]
pub extern "C" fn parse_syntax(
    result: &mut CFlattenedSyntaxTree,
    string_len: u64, string_ptr: *mut u8,
    mode: i32,
) {
    let string = RawString::new(string_len, string_ptr);
    let input = string.into_string().unwrap();
    let node = match mode {
        0 => parse(input.as_str()),      // Content
        1 => parse_code(input.as_str()), // Code
        2 => parse_math(input.as_str()), // Math
        _ => panic!("Unexpected mode {} for syntax", mode),
    };
    let flattened = flattened_tree(node);
    let marks: Vec<i64> = flattened
        .marks
        .iter()
        .map(|it| ((it.0.encode() as i64) << 32) + it.1 as i64)
        .collect();
    result.marks = marks.into();
    result.errors = flattened.errors.into();
    result.errors_starts = flattened.errors_starts.into();
}

#[unsafe(no_mangle)]
pub extern "C" fn allocate_flattened_tree() -> *const CFlattenedSyntaxTree {
    let tree = CFlattenedSyntaxTree {
        marks: CVec::default(),
        errors: CVec::default(),
        errors_starts: CVec::default(),
    };
    Box::into_raw(Box::new(tree))
}

#[unsafe(no_mangle)]
pub extern "C" fn release_flattened_tree(tree: *mut CFlattenedSyntaxTree) {
    let tree = *unsafe { Box::from_raw(tree) };
    let _marks: Vec<i64> = tree.marks.into();
    let _errors: Vec<u8> = tree.errors.into();
    let _errors_starts: Vec<i32> = tree.errors_starts.into();
}
