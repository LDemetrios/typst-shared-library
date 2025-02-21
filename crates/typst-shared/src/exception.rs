use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct Exception {
    pub class: String,
    pub message: Option<String>,
    pub cause: Option<Arc<Exception>>,
    pub stack_trace: Vec<StackTraceElement>,
    pub suppressed: Vec<Arc<Exception>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StackTraceElement {
    pub class_loader_name: Option<String>,
    pub module_name: Option<String>,
    pub module_version: Option<String>,
    pub declaring_class: Option<String>,
    pub method_name: Option<String>,
    pub file_name: Option<String>,
    pub line_number: u32,
}

pub type Except<T> = Result<T, Exception>;

#[macro_export]
macro_rules! here {
    () => {{
        fn f() {}
        let full_name = std::any::type_name_of_val(&f)
            .split("::")
            .skip(2)
            .collect::<Vec<_>>()
            .split_last()
            .map(|(_, rest)| rest.join("::"))
            .unwrap();

        let (class, method) = match full_name.rfind("::") {
            Some(index) => {
                let (first, second) = full_name.split_at(index);
                (Some(first.to_string()), second[2..].to_string()) // Skip "::"
            }
            None => (None, full_name.to_string()),
        };
        $crate::exception::StackTraceElement {
            class_loader_name: Some("TypstSharedLibrary".to_string()),
            module_name: Some(env!("CARGO_PKG_NAME").to_string()),
            module_version: None,
            declaring_class: class,
            method_name: Some(method),
            file_name: Some(file!().to_string()),
            line_number: line!(),
        }
    }};
}

#[macro_export]
macro_rules! throw {
    ($class: expr) => {
        throw!($class, None, None)
    };
    ($class: expr, $message: expr) => {
        throw!($class, $message, None)
    };
    ($class: expr, $message: expr, $cause: expr) => {
        $crate::exception::Exception {
            class: ($class),
            message: ($message),
            cause: ($cause),
            stack_trace: vec![$crate::here!()],
            suppressed: vec![],
        }
    };
}

#[macro_export]
macro_rules! add_frame {
    ($inside: expr) => {{
        match ($inside) {
            Ok(r) => Ok(r),
            Err(mut exc) => {
                value.stack_trace.push($crate::exception::here!());

                Err(exc)
            }
        }
    }};
}

#[macro_export]
macro_rules! or_rethrow {
    ($inside: expr) => {{
        match ($inside) {
            Ok(r) => Ok(r),
            Err(mut exc) => {
                value.stack_trace.push($crate::exception::here!());
                return Err(exc)
            }
        }
    }};
}