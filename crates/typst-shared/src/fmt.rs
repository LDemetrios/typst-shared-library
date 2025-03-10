use crate::memory_management::ThickBytePtr;
use typstyle_core::{Config, Typstyle};
use typst::utils::tick;

#[no_mangle]
pub extern "C" fn format_source(content: ThickBytePtr, column: i32, tab_width: i32) -> ThickBytePtr {
    tick!("{:?}, {}, {}", content, column, tab_width);
    let str = content.to_str();
    let result = format(str, column, tab_width);
    ThickBytePtr::from_str(result)
}

pub fn format(content: String, column: i32, tab_width: i32) -> String {
    let cfg = Config::new()
        .with_width(column as usize)
        .with_tab_spaces(tab_width as usize);
    Typstyle::new(cfg).format_content(&content).unwrap_or_else(|err| "".to_string())
}
