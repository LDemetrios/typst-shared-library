#![allow(dead_code)]
// use crate::extended_info::{ExtendedFileDescriptor, ExtendedFileResult, Resolve};
// use crate::java_world::{FileCache, FileCallback, JavaWorld, MainCallback, Now};
// use crate::memory_management::{Base16ByteArray, JavaResult, ThickBytePtr};
// use parking_lot::Mutex;
// use std::collections::HashMap;
use std::path::PathBuf;
use typst::comemo::Track;
use typst::ecow::EcoVec;
use typst::syntax::{FileId, Source, Span};
use typst::utils::LazyHash;
use typst_eval::eval_string;
use typst_kit::fonts::{FontSlot, Fonts};
// use typst_kit::package::PackageStorage;
use typst_library::diag::{FileResult, SourceDiagnostic};
use typst_library::foundations::{Bytes, Datetime, Dict, Scope, Value};
use typst_library::routines::EvalMode;
use typst_library::{Library, World};
use typst_library::text::{Font, FontBook};

pub mod cache_cell;
pub mod compile;
pub mod detached_eval;
pub mod download;
pub mod exception;
pub mod extended_info;
pub mod fmt;
pub mod java_world;
pub mod memory_management;
pub mod query;
pub mod stdlib;
pub mod syntax;
pub mod terminal;

// pub extern "C" fn main_nop() -> JavaResult<ExtendedFileDescriptor> {
//     panic!()
// }
//
// pub extern "C" fn file_nop(
//     _it: ThickBytePtr,
// ) -> JavaResult<ExtendedFileResult<Base16ByteArray>> {
//     panic!()
// }

pub struct SimplifiedWorld {
    pub library: LazyHash<Library>,
    pub book: LazyHash<FontBook>,
    pub fonts: Vec<FontSlot>,
}

impl World for SimplifiedWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        unreachable!()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        unreachable!()
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        unreachable!()
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts[index].get()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        None
    }
}
fn main() {
    let lib = Library::builder().with_inputs(Dict::default()).build();

    let fonts = Fonts::searcher()
        .include_system_fonts(true)
        .search_with(&(vec![] as Vec<PathBuf>));

    // let mut world = JavaWorld {
    //     library: LazyHash::new(lib),
    //     book: LazyHash::new(fonts.book),
    //     main_callback: main_nop,
    //     file_callback: file_nop,
    //     fonts: fonts.fonts,
    //     files: Mutex::new(HashMap::new()),
    //     now: None,
    //     package_storage: Some(PackageStorage::new(
    //         None,
    //         None,
    //         download::downloader(),
    //     )),
    //     auto_load_central: false,
    // };
    let mut world = SimplifiedWorld {
            library: LazyHash::new(lib),
            book: LazyHash::new(fonts.book),
            fonts: fonts.fonts,
    };
    let lim = 20000000;
    let source = "1 + 2";
    for i in 0..lim {
        // world.reset(); -- Doesn't do anything without file stuff
        let _result = eval(&world, source);
        // comemo::evict(1);
    }
}

fn eval(world: &dyn World, source: &str) -> Result<Value, EcoVec<SourceDiagnostic>> {
    eval_string(
        &typst::ROUTINES,
        world.track(),
        source,
        Span::detached(),
        EvalMode::Code,
        Scope::default(),
    )
}

