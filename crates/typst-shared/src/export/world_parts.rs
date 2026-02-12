// Added by LDemetrios

use crate::export::raw_bytes::Base64Bytes;
use crate::export::raw_string::RawString;
use crate::export::utils::{evaluate, to_file_id_or_none};
use crate::serial::extended_info::{ExtendedFileDescriptor, ExtendedFileResult};
use crate::serial::extended_info::{ExtendedSourceDiagnostic, ExtendedSpan, Resolve};
use crate::world::composite::CompositeWorld;
use crate::world::files::{FilesCache, ReadCallback};
use crate::world::fonts::FontCollection;
use crate::world::library::{replace_inputs, stdlib};
use crate::world::packages::PackageManager;
use crate::world::register_test_definitions;
use typst::syntax::{FileId, Source, SyntaxMode};
use typst::utils::{LazyHash, SmallBitSet};
use typst_library::diag::{FileError, FileResult, Severity};
use typst_library::foundations::{Bytes, Datetime, Value};
use typst_library::text::{Font, FontBook};
use typst_library::{Library, World};

#[macro_export]
macro_rules! free_func {
    ($fn_name:ident, $type_name:ty) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $fn_name(__ptr: *mut $type_name) {
            if __ptr.is_null() {
                return;
            }
            unsafe { drop(Box::from_raw(__ptr)) };
        }
    };
}

// files: &'static FilesCache<Reader>

unsafe extern "C" {
    pub fn read_file_by_reader_ticket(
        result: &mut RawString,
        ticket: i64,
        coordinates_len: u64,
        coordinates_ptr: *mut u8,
    );
}

#[derive(Clone, Copy, Debug)]
pub struct TicketedReader(i64);

impl ReadCallback for TicketedReader {
    fn read(&self, id: FileId) -> FileResult<Vec<u8>> {
        let id_w: ExtendedFileDescriptor = id.into();
        let arg = RawString::from_value(&id_w);
        let result = unsafe {
            let mut res = RawString::default();
            read_file_by_reader_ticket(&mut res, self.0, arg.len, arg.ptr as *mut u8);
            res
        };
        arg.release();
        result
            .read_to::<ExtendedFileResult<Base64Bytes>>()
            .map(|it| it.0)
            .map_err(|it| it.into())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn files_cache(reader_ticket: i64) -> *const FilesCache<TicketedReader> {
    Box::into_raw(Box::new(FilesCache::new(TicketedReader(reader_ticket))))
}

free_func!(free_files_cache, FilesCache<TicketedReader>);

#[unsafe(no_mangle)]
pub extern "C" fn resolve_preview_package(
    result: &mut RawString,
    id_len: u64,
    id_ptr: *mut u8,
) {
    let id = RawString::new(id_len, id_ptr);
    let descriptor = id.read_to::<ExtendedFileDescriptor>();
    let result_v: ExtendedFileResult<Base64Bytes> = match descriptor.pack.clone() {
        Some(_) => {
            let packages = PackageManager::new(None, None);
            packages.load(descriptor.into()).map(Base64Bytes)
        }
        None => Err(FileError::NotFound(descriptor.path.into())),
    }
    .map_err(|it| it.into());
    RawString::write_from(result, &result_v);
}

#[unsafe(no_mangle)]
pub extern "C" fn reset_file(
    cache: &mut FilesCache<TicketedReader>,
    id_len: u64,
    id_ptr: *mut u8,
) {
    let id = RawString::new(id_len, id_ptr);
    if let Some(file) = to_file_id_or_none(id) {
        cache.reset(file);
    }
}

// fonts: &'static FontCollection,

#[unsafe(no_mangle)]
pub extern "C" fn font_collection(
    include_system: i32,
    include_embedded: i32,
    font_paths_len: u64,
    font_paths_ptr: *mut u8,
) -> *const FontCollection {
    let font_paths = RawString::new(font_paths_len, font_paths_ptr);
    let paths = font_paths.read_to::<Vec<String>>();
    let collection =
        FontCollection::new(include_system != 0, include_embedded != 0, paths);
    Box::into_raw(Box::new(collection))
}

free_func!(free_font_collection, FontCollection);

// library: &'static LazyHash<Library>

#[unsafe(no_mangle)]
pub extern "C" fn library(features: i32) -> *mut LazyHash<Library> {
    let mut features_bitset = SmallBitSet::default();
    for i in 0..2 {
        if features >> i & 1 == 1 {
            features_bitset.insert(i as usize)
        }
    }
    Box::into_raw(Box::new(LazyHash::new(stdlib(features_bitset))))
}

#[unsafe(no_mangle)]
pub extern "C" fn with_inputs(
    result: &mut RawString,
    fonts: *const FontCollection,
    library: &mut LazyHash<Library>,
    inputs_len: u64,
    inputs_ptr: *mut u8,
    close_previous: i32,
) {
    let inputs = RawString::new(inputs_len, inputs_ptr);
    let inputs = inputs.into_string().unwrap();
    let mut library = if close_previous != 0 {
        *unsafe { Box::from_raw(library) }
    } else {
        library.clone()
    };
    let stub: CompositeWorld<TicketedReader> =
        CompositeWorld::new(None, const_ref(fonts), Some(&library), None, None);
    let result_v: Result<i64, Vec<ExtendedSourceDiagnostic>> =
        match evaluate(&stub, inputs.as_str(), SyntaxMode::Code) {
            Err(err) => Err(err.resolve(&stub)),
            Ok(Value::Dict(val)) => {
                let mut library_inner = library.into_inner();
                replace_inputs(&mut library_inner, val);
                Ok(Box::into_raw(Box::new(LazyHash::new(library_inner))) as _)
            }
            _ => detached_diagnostic("Evaluated `inputs` is not a dict"),
        };
    RawString::write_from(result, &result_v);
}

#[unsafe(no_mangle)]
pub extern "C" fn ext_with_test_definitions(
    library: &mut LazyHash<Library>,
    close_previous: i32,
) -> i64 {
    let mut library = if close_previous != 0 {
        *unsafe { Box::from_raw(library) }
    } else {
        library.clone()
    }
    .into_inner();
    register_test_definitions(&mut library);
    Box::into_raw(Box::new(LazyHash::new(library))) as *const LazyHash<Library> as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn with_styles(
    result: &mut RawString,
    fonts: *const FontCollection,
    library: &mut LazyHash<Library>,
    styles_len: u64,
    styles_ptr: *mut u8,
    close_previous: i32,
    append: i32,
) {
    let styles = RawString::new(styles_len, styles_ptr);
    let styles = styles.into_string().unwrap();
    let mut library = if close_previous != 0 {
        *unsafe { Box::from_raw(library) }
    } else {
        library.clone()
    };
    let stub: CompositeWorld<TicketedReader> =
        CompositeWorld::new(None, const_ref(fonts), Some(&library), None, None);
    let result_v: Result<i64, Vec<ExtendedSourceDiagnostic>> =
        match evaluate(&stub, styles.as_str(), SyntaxMode::Code) {
            Err(err) => Err(err.resolve(&stub)),
            Ok(Value::Content(val)) => {
                if let Ok(Value::Styles(styles)) = val.field_by_name("styles") {
                    let library_inner = library.into_inner();
                    let new_styles = if append != 0 {
                        styles
                            .into_iter()
                            .chain(library_inner.styles.into_iter())
                            .collect::<typst_library::foundations::Styles>()
                    } else {
                        styles
                    };
                    let new_lib = Library { styles: new_styles, ..library_inner };

                    Ok(Box::into_raw(Box::new(LazyHash::new(new_lib))) as _)
                } else {
                    detached_diagnostic("Evaluated `styles` is not a styled")
                }
            }
            _ => detached_diagnostic("Evaluated `styles` is not a content"),
        };
    RawString::write_from(result, &result_v);
}

pub fn detached_diagnostic<T>(message: &str) -> Result<T, Vec<ExtendedSourceDiagnostic>> {
    Err(vec![ExtendedSourceDiagnostic {
        severity: Severity::Error,
        span: ExtendedSpan {
            native: 0,
            file: None,
            start_ind: 0,
            end_ind: 0,
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        },
        message: message.to_string(),
        trace: vec![],
        hints: vec![],
    }])
}

free_func!(free_lazy_hash_library, LazyHash<Library>);

pub fn mut_ref<T>(value: *mut T) -> Option<&'static mut T> {
    if value as usize == 0 {
        None
    } else {
        Some(Box::leak(unsafe { Box::from_raw(value) }))
    }
}

pub fn const_ref<T>(value: *const T) -> Option<&'static T> {
    if value as usize == 0 {
        None
    } else {
        Some(Box::leak(unsafe { Box::from_raw(value as *mut T) }))
    }
}

pub struct NoopWorld {}

impl World for NoopWorld {
    fn library(&self) -> &LazyHash<Library> {
        unimplemented!()
    }

    fn book(&self) -> &LazyHash<FontBook> {
        unimplemented!()
    }

    fn main(&self) -> FileId {
        unimplemented!()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        unimplemented!()
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        unimplemented!()
    }

    fn font(&self, index: usize) -> Option<Font> {
        None
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        None
    }
}
