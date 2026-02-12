// Added by LDemetrios

use crate::Now;

#[repr(C)]
pub struct RawNow {
    millis_or_flag: i64, // -1 to None, -2 to System
    nanos: i32,
}

impl RawNow {
    pub fn new(millis_or_flag: i64, nanos: i32) -> Self {
        Self { millis_or_flag, nanos }
    }
    pub fn resolve(self) -> Option<Now> {
        match self.millis_or_flag {
            -1 => None,
            -2 => Some(Now::System),
            _ => Some(Now::Fixed {
                millis: self.millis_or_flag,
                nanos: self.nanos,
            }),
        }
    }
}