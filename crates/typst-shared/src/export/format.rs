// Added by LDemetrios

use crate::export::raw_string::RawString;
use typstyle_core::{Config, Typstyle};

#[unsafe(no_mangle)]
pub extern "C" fn format_source(result:&mut RawString, content_len: u64, content_ptr: *mut u8, column: i32, tab_width: i32) {
    let content = RawString::new(content_len, content_ptr);
    let str = content.into_string().unwrap();
    let cfg = Config::new()
        .with_width(column as usize)
        .with_tab_spaces(tab_width as usize);
    let result_v = Typstyle::new(cfg).format_content(str).unwrap_or_else(|_err| "".to_string());

    *result = RawString::from_string(result_v);
}
