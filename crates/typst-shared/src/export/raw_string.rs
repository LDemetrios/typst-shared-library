// Added by LDemetrios

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::mem;
use std::ptr::null_mut;
use std::slice;
use std::str;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub(crate) struct RawString {
    pub len: u64,
    pub ptr: u64,
}

impl Default for RawString {
    fn default() -> Self {
        RawString { len: 0, ptr: null_mut::<u8>() as u64 }
    }
}

impl RawString {
    pub fn new(len: u64, ptr: *mut u8) -> Self {
        RawString { len, ptr: ptr as u64 }
    }

    pub fn into_string(self) -> Option<String> {
        if self.len == 0 {
            return Some(String::new());
        } else if (self.ptr as *mut u8).is_null() {
            return None;
        }

        unsafe {
            let vec = Vec::from_raw_parts(
                self.ptr as *mut u8,
                self.len as usize,
                self.len as usize,
            );
            Some(String::from_utf8_unchecked(vec))
        }
    }

    pub fn from_string(s: String) -> Self {
        let mut s = s;
        let len = s.len();
        let ptr = s.as_mut_ptr();

        mem::forget(s);

        RawString::new(len as u64, ptr)
    }

    pub fn as_str(&self) -> Option<&str> {
        if (self.ptr as *mut u8).is_null()  {
            return if self.len == 0 {
                Some("")
            } else {
                None
            }
        }

        unsafe {
            let slice = slice::from_raw_parts(self.ptr as *mut u8, self.len as usize);
            Some(str::from_utf8_unchecked(slice))
        }
    }

    pub fn release(self) {
        drop(unsafe {
            Vec::from_raw_parts(self.ptr as *mut u8, self.len as usize, self.len as usize)
        })
    }

    pub fn write_from<T: Serialize>(&mut self, value: &T) {
        let result = Self::from_value(value);
        self.len = result.len;
        self.ptr = result.ptr;
    }

    pub fn from_value<T: Serialize>(value: &T) -> RawString {
        Self::from_string(
            serde_json::to_string(&value).expect("FATAL: error serializing value"),
        )
    }

    pub fn read_as<T: DeserializeOwned>(&self) -> T {
        let s = self.as_str().unwrap();
        serde_json::from_str(s).unwrap()
    }

    pub fn read_to<T: DeserializeOwned>(self) -> T {
        let s = self.into_string().unwrap();
        serde_json::from_str(s.as_str()).unwrap()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn allocate_raw_string() -> *const RawString {
    Box::into_raw(Box::new(RawString { len: 0u64, ptr: null_mut::<u8>() as u64 }))
}

#[unsafe(no_mangle)]
pub extern "C" fn allocate_ptr_for_raw_string(size: i64) -> *const u8 {
    if size <= 0 {
        return std::ptr::null();
    }

    let mut vec = Vec::with_capacity(size as usize);
    let ptr = vec.as_mut_ptr();
    mem::forget(vec);
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn free_raw_string(str: *mut RawString) {
    let str = *unsafe { Box::from_raw(str) };
    str.release();
}
