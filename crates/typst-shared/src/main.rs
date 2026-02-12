// Added by LDemetrios

use std::backtrace::Backtrace;
use std::panic;

pub mod export;
pub mod serial;
pub mod syntax;
pub mod terminal;
pub mod world;

pub use export::query::*;

use crate::world::{stdlib, CompositeWorld, FilesCache, FontCollection, ReadCallback};
use typst::syntax::{FileId, RootedPath, VirtualPath, VirtualRoot};
use typst::utils::{LazyHash, SmallBitSet};
use typst_library::diag::FileResult;
pub use world::time::Now;
use crate::serial::extended_info::{ExtendedSourceDiagnostic, ExtendedWarned};

pub use self::{
    export::*,
    serial::*,
    syntax::*,
    terminal::*,
};

#[derive(Debug, Copy, Clone)]
struct X {}
impl ReadCallback for X {
    fn read(&self, _id: FileId) -> FileResult<Vec<u8>> {
        Ok(content.to_string().into_bytes())
    }
}
const  content: &str = "#link(text(\"link to heading\",),label(\"heading\",),)";
// "#[a\\ ].func()(([ ],[#(heading(text(\"Heading\",),depth:1,))#label(\"heading\",)],[ ],parbreak(),link(text(\"link to heading\",),label(\"heading\",),),parbreak(),),)";

fn main() {
    let fonts = FontCollection::new(true, true, vec![]);
    let context = FilesCache::new(X{});
    let stdlib = LazyHash::new(stdlib(SmallBitSet::new()));

    let world = CompositeWorld::new(
        Some(&context),
        Some(&fonts),
        Some(&stdlib),
        Some(FileId::new(RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new("/main.typ").expect("invalid virtual path"),
        ))),
        Some(Now::System),
    );

    let result_v: ExtendedWarned<
        Result<Vec<()>, Vec<ExtendedSourceDiagnostic>>,
    > = crate::export::compile::compile_paged(&world, 0, 1, |page| {
        let pixmap = typst_render::render(page, (144.0 / 72.0) as f32);

        let buf = pixmap.encode_png().unwrap();

        ()
    });

    println!("{:#?}", result_v);
}
