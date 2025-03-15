use crate::java_world::JavaWorld;
use std::ffi::CString;
use std::marker::PhantomData;
use std::mem;
use std::os::raw::c_char;
use std::ptr::{null, null_mut};
use typst::Library;

use crate::exception::Except;
use hex::{decode, encode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::OnceLock;
use typst::utils::tick;

#[macro_export]
macro_rules! free_fn {
    ($fn_name:ident, $type_name:ty) => {
        #[no_mangle]
        pub extern "C" fn $fn_name(__ptr: *mut $type_name) {
            tick!();
            if __ptr.is_null() {
                return;
            }
            unsafe { drop(Box::from_raw(__ptr)) };
        }
    };
}

free_fn!(free_library, Library);
free_fn!(free_world, JavaWorld);

#[no_mangle]
pub extern "C" fn free_str(ptr: *mut c_char) {
    tick!();
    if ptr.is_null() {
        return;
    }
    unsafe { drop(CString::from_raw(ptr)) };
}

static FREER: OnceLock<extern "C" fn(ticket: i64)> = OnceLock::new();

#[no_mangle]
pub extern "C" fn set_freer(f: extern "C" fn(i64)) -> i32 {
    match FREER.set(f) {
        // Ok(_) => Ok(()),
        // Err(_) => throw!(
        //     "java.lang.IllegalStateException".to_string(),
        //     Some("Can't reset freer function")
        // ),
        Ok(_) => 1,
        Err(_) => 0,
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct JavaResult<T: Sized> {
    pub ticket: i64,
    pub value: ThickBytePtr,
    pub phantom: PhantomData<T>,
}

impl<T: for<'a> Deserialize<'a>> JavaResult<T> {
    pub fn unpack(self) -> T {
        tick!();
        let Self { ticket, value, phantom: _phantom } = self;

        tick!();
        if ticket >= 0 {
            FREER.get().unwrap()(ticket);
        }
        tick!();

        let str = value.to_str();
        tick!("{}", str);
        let result = serde_json::from_str::<T>(str.as_str()).unwrap();
        mem::forget(str);
        result
    }
}

impl<T: Serialize> JavaResult<T> {
    pub fn pack(value: T) -> JavaResult<T> {
        let str = serde_json::to_string(&value).expect("FATAL: error serializing value");
        JavaResult {
            ticket: 0,
            value: ThickBytePtr::from_str(str),
            phantom: PhantomData,
        }
    }
}

#[repr(C)]
pub struct JavaExceptPtrResult<T> {
    pub comment: ThickBytePtr,
    pub ptr: *const T,
}

impl<T> JavaExceptPtrResult<T> {
    pub fn pack(value: Except<*const T>) -> Self {
        match value {
            Ok(v) => JavaExceptPtrResult { comment: ThickBytePtr::null(), ptr: v },
            Err(e) => {
                let str =
                    serde_json::to_string(&e).expect("FATAL: error serializing value");

                JavaExceptPtrResult { comment: ThickBytePtr::from_str(str), ptr: null() }
            }
        }
    }
}


#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CVec<T> {
    pub ptr: *mut T,
    pub len: i64,
    pub cap: i64,
}

impl<T> From<Vec<T>> for CVec<T> {
    fn from(value: Vec<T>) -> Self {
        let res = CVec {
            ptr: value.as_ptr() as *mut T,
            len: value.len() as i64,
            cap: value.capacity() as i64,
        };
        mem::forget(value);
        res
    }
}

impl<T> From<CVec<T>> for Vec<T> {
    fn from(value: CVec<T>) -> Self {
        unsafe { Vec::from_raw_parts(value.ptr, value.len as usize, value.cap as usize) }
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct ThickBytePtr(pub CVec<u8>);

impl From<String> for ThickBytePtr {
    fn from(value: String) -> Self {
        Self::from_str(value)
    }
}

impl ThickBytePtr {
    pub fn null() -> Self {
        ThickBytePtr(CVec { ptr: null_mut(), len: 0, cap: 0 })
    }

    pub fn from_str(mut str: String) -> Self {
        let len = str.len();
        let ptr = str.as_mut_ptr();
        let cap = str.capacity();
        std::mem::forget(str);
        ThickBytePtr ( CVec{
            ptr,
            len: len as i64,
            cap: cap as i64,
        } )
    }

    pub fn to_str(self) -> String {
        tick!("{:?}", self);
        let CVec { ptr, len, cap } = self.0;
        tick!();
        unsafe {
            String::from_raw_parts(ptr, len as usize, cap as usize) /*Vec::from_raw_parts(ptr, len as usize, 0)*//* */
        }
    }

    pub fn release(self) {
        // let Self { ptr, len } = self;
        // tick!();
        //  unsafe { drop(Vec::from_raw_parts(ptr, len as usize, 0) )};
        drop(self.to_str())
    }
}

#[derive(Debug)]
pub struct Base16ByteArray(pub Vec<u8>);

impl Serialize for Base16ByteArray {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex_string = encode(&self.0);
        serializer.serialize_str(&hex_string)
    }
}

impl<'de> Deserialize<'de> for Base16ByteArray {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hex_str = String::deserialize(deserializer)?;
        let bytes = decode(&hex_str).map_err(serde::de::Error::custom)?;
        Ok(Base16ByteArray(bytes))
    }
}

impl From<Base16ByteArray> for Vec<u8> {
    fn from(value: Base16ByteArray) -> Self {
        value.0
    }
}

impl From<Vec<u8>> for Base16ByteArray {
    fn from(value: Vec<u8>) -> Self {
        Base16ByteArray(value)
    }
}

#[no_mangle]
extern "C" fn free_thick_byte_ptr(ptr: ThickBytePtr) {
    ptr.release()
}
