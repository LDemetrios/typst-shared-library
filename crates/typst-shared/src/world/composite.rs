// Added by LDemetrios

use std::fmt::Debug;
use crate::world::files::FilesCache;
use crate::world::files::ReadCallback;
use crate::world::fonts::FontCollection;
use crate::world::time::{Now, TimeHolder};
use typst::syntax::{FileId, Source};
use typst::utils::{tick, LazyHash};
use typst_library::diag::FileResult;
use typst_library::foundations::{Bytes, Datetime};
use typst_library::text::{Font, FontBook};
use typst_library::{Library, World};

pub struct CompositeWorld<'a, Reader: ReadCallback + Clone + Copy+ Debug+'a> {
    files: Option<&'a FilesCache<Reader>>,
    fonts: Option<&'a FontCollection>,
    library: Option<&'a LazyHash<Library>>,
    main: Option<FileId>,
    time: Option<TimeHolder>,
    session: i64,
}

impl<'a, Reader: ReadCallback + Clone+ Copy+ Debug + 'a> CompositeWorld<'a, Reader> {
    pub fn new(
        files: Option<&'a FilesCache<Reader>>,
        fonts: Option<&'a FontCollection>,
        library: Option<&'a LazyHash<Library>>,
        main: Option<FileId>,
        time: Option<Now>,
        session: i64,
    ) -> Self {
        Self {
            files,
            fonts,
            library,
            main,
            time: time.map(TimeHolder::new),
            session,
        }
    }
}

impl<'a, Reader: ReadCallback + Clone + Copy + Debug + 'a + Send + Sync> World
    for CompositeWorld<'a, Reader>
{
    fn library(&self) -> &LazyHash<Library> {
        self.library.unwrap()
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.unwrap().book()
    }

    fn main(&self) -> FileId {
        self.main.unwrap()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {

        self.files.unwrap().source(id)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.unwrap().file(id)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.unwrap().font(index)
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        self.time.as_ref().and_then(|it| it.today(offset))
    }

    fn session(&self) -> i64 {
        self.session
    }
}
