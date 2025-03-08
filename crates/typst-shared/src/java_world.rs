use chrono::{DateTime, Datelike, FixedOffset, Local, TimeZone, Timelike, Utc};

use crate::cache_cell::CacheCell;
use crate::download;
use crate::download::PrintDownload;
use crate::extended_info::{
    ExtendedFileDescriptor, ExtendedFileResult, Resolve,
};
use crate::memory_management::{
    Base16ByteArray, JavaExceptPtrResult, JavaResult, ThickBytePtr,
};
use parking_lot::Mutex;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fs;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::OnceLock;
use typst::diag::FileResult;
use typst::foundations::{
    Bytes, Datetime,
};
use typst::syntax::package::PackageSpec;
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::{tick, LazyHash};
use typst::{Library, World};
use typst_kit::fonts::{FontSlot, Fonts};
use typst_kit::package::PackageStorage;
use typst_library::diag::FileError;

pub type MainCallback = extern "C" fn() -> JavaResult<ExtendedFileDescriptor>;
pub type FileCallback =
    extern "C" fn(ThickBytePtr) -> JavaResult<ExtendedFileResult<Base16ByteArray>>;

/// JavaWorld keeps anything that is needed to impl World from java code with JNA.
/// It is not directly representable with JNA, therefore no #[repr(C)],
/// and JavaWorld is stored and accessed by Pointer
pub struct JavaWorld {
    /// Typst's standard library.
    pub(crate) library: LazyHash<Library>,
    /// Metadata about discovered fonts. TODO make java-compatible
    pub(crate) book: LazyHash<FontBook>,
    /// Callback for World::book method.
    /// Returns c-style string representing a path to a main file.
    pub(crate) main_callback: MainCallback,
    /// Callback for World::file method.
    /// Accepts package: Option<PackageSpec> and path: VirtualPath
    /// Return FileResult<Bytes>
    pub(crate) file_callback: FileCallback,
    /// Fonts, handled as in SystemWorld. TODO make java-compatible
    pub(crate) fonts: Vec<FontSlot>,
    /// File cache
    pub(crate) files: Mutex<HashMap<FileId, FileCache>>,
    /// Now, handled as in SystemWorld
    pub(crate) now: Option<Now>,
    /// Package storage, handled as in SystemWorld
    pub(crate) package_storage: Option<PackageStorage>,
    pub auto_load_central: bool,
}

pub enum Now {
    /// The date and time if the environment `SOURCE_DATE_EPOCH` is set.
    /// Used for reproducible builds.
    Fixed { stamp: DateTime<Utc> },
    /// The current date and time if the time is not externally fixed.
    System { locked: OnceLock<DateTime<Utc>> },
}

impl<'de> Deserialize<'de> for Now {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self};
        #[derive(Deserialize)]
        #[serde(tag = "type")]
        enum Helper {
            Fixed { millis: i64, nanos: i32 },
            System,
        }

        let helper = Helper::deserialize(deserializer)?;
        Ok(match helper {
            Helper::Fixed { millis, nanos } => {
                let stamp = Utc
                    .timestamp_millis_opt(millis)
                    .single()
                    .ok_or_else(|| de::Error::custom("invalid timestamp"))?
                    .with_nanosecond(nanos as u32)
                    .ok_or_else(|| de::Error::custom("invalid nanoseconds"))?;
                Now::Fixed { stamp }
            }
            Helper::System => Now::System { locked: OnceLock::new() },
        })
    }
}

pub struct FileCache {
    /// The slot's file id.
    id: FileId,
    /// The lazily loaded and incrementally updated source file.
    source: CacheCell<Source>,
    /// The lazily loaded raw byte buffer.
    file: CacheCell<Bytes>,
}

#[no_mangle]
pub extern "C" fn new_world(
    library: *mut Library,
    main_callback: MainCallback,
    file_callback: FileCallback,
    now: JavaResult<Option<Now>>,
    auto_load_central: i32, // 1 -- true, 0 -- false
) -> JavaExceptPtrResult<JavaWorld> {
    tick!();
    let library = unsafe { Box::from_raw(library) }.deref().clone();
    tick!();

    let fonts = Fonts::searcher()
        .include_system_fonts(true)
        .search_with(&(vec![] as Vec<PathBuf>));
    tick!();

    let package_cache_path: Option<PathBuf> = None;
    let package_path: Option<PathBuf> = None;

    let java_world = JavaWorld {
        library: LazyHash::new(library),
        book: LazyHash::new(fonts.book),
        main_callback,
        file_callback,
        fonts: fonts.fonts,
        files: Mutex::new(HashMap::new()),
        now: now.unpack().into(),
        package_storage: Some(PackageStorage::new(
            package_cache_path.clone(),
            package_path.clone(),
            download::downloader(),
        )),
        auto_load_central: auto_load_central == 1,
    };
    tick!();
    JavaExceptPtrResult::pack(Ok(Box::into_raw(Box::new(java_world))))
}

impl FileCache {
    fn new(id: FileId) -> Self {
        Self {
            id,
            file: CacheCell::new(),
            source: CacheCell::new(),
        }
    }

    fn accessed(&self) -> bool {
        self.source.accessed() || self.file.accessed()
    }

    fn reset(&mut self) {
        self.source.reset();
        self.file.reset();
    }
}

impl JavaWorld {
    fn cell<F, T>(&self, id: FileId, f: F) -> T
    where
        F: FnOnce(&mut FileCache) -> T,
    {
        let mut map = self.files.lock();
        f(map.entry(id).or_insert_with(|| FileCache::new(id)))
    }

    pub fn reset(&mut self) {
        for slot in self.files.get_mut().values_mut() {
            slot.reset();
        }
        if let Some(Now::System { locked }) = &mut self.now {
            locked.take();
        }
    }

    pub fn obtain_file(&self, id: FileId) -> FileResult<Vec<u8>> {
        let custom: bool;

        if let Some(pack) = id.package() {
            let PackageSpec { namespace, .. } = pack;
            custom = !namespace.to_string().eq(&"preview".to_string());
        } else {
            custom = true
        }

        if custom {
            let descriptor: ThickBytePtr =
                serde_json::to_string(&ExtendedFileDescriptor::from(id))
                    .unwrap()
                    .into();
            tick!();
            let jr = (self.file_callback)(descriptor);
            tick!("{:?}", jr);
            let result =
                jr.unpack().map(|it| it.into()).map_err(|it| it.into());
            tick!();
            descriptor.release();
            tick!();
            result
        } else {
            let spec = id.package().unwrap();
            let buf = self
                .package_storage
                .as_ref()
                .unwrap()
                .prepare_package(spec, &mut PrintDownload(&spec))?;
            let mut root = &buf;
            let path =
                id.vpath().resolve(root).ok_or(FileError::AccessDenied);
            read_from_disk(&path?)
        }
    }
}

#[no_mangle]
pub extern "C" fn reset_world(world_ptr: *mut JavaWorld) {
    let mut world = unsafe { Box::from_raw(world_ptr) };
    world.reset();
    let _ = Box::into_raw(world); // Not to drop the world!
}

impl World for JavaWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        let x = JavaResult::unpack((self.main_callback)());
        tick!("{:?}", x);
        x.into()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.cell(id, |it| {
            it.source.get_or_init(
                || {
                    self.obtain_file(id)
                },
                |data, prev| {
                    let text = decode_utf8(&data)?;
                    if let Some(mut prev) = prev {
                        prev.replace(text);
                        Ok(prev)
                    } else {
                        Ok(Source::new(id, text.into()))
                    }
                },
            )
        })
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        tick!();
        self.cell(id, |it| {
            it.file.get_or_init(
                || {
                    self.obtain_file(id)
                },
                |data, _| Ok(Bytes::new(data)),
            )
        })
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts[index].get()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let t = match &self.now {
            Some(x) => x,
            None => return None,
        };
        let now = match &t {
            Now::Fixed { stamp } => stamp,
            Now::System { locked } => locked.get_or_init(Utc::now),
        };

        // The time with the specified UTC offset, or within the local time zone.
        let with_offset = match offset {
            None => now.with_timezone(&Local).fixed_offset(),
            Some(hours) => {
                let seconds = i32::try_from(hours).ok()?.checked_mul(3600)?;
                now.with_timezone(&FixedOffset::east_opt(seconds)?)
            }
        };

        Datetime::from_ymd(
            with_offset.year(),
            with_offset.month().try_into().ok()?,
            with_offset.day().try_into().ok()?,
        )
    }
}

fn decode_utf8(buf: &[u8]) -> FileResult<&str> {
    Ok(std::str::from_utf8(buf.strip_prefix(b"\xef\xbb\xbf").unwrap_or(buf))?)
}

fn read_from_disk(path: &Path) -> FileResult<Vec<u8>> {
    let f = |e| FileError::from_io(e, path);
    if fs::metadata(path).map_err(f)?.is_dir() {
        Err(FileError::IsDirectory)
    } else {
        fs::read(path).map_err(f)
    }
}
