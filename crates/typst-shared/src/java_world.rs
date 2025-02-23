use chrono::{DateTime, Datelike, FixedOffset, Local, TimeZone, Timelike, Utc};

use crate::cache_cell::CacheCell;
use crate::extended_info::{
    ExtendedFileDescriptor, ExtendedFileResult, ExtendedWarned, Resolve,
};
use crate::memory_management::{
    Base16ByteArray, JavaExceptPtrResult, JavaResult, ThickBytePtr,
};
use parking_lot::Mutex;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::io::Write;
use std::ops::Deref;
use std::os::raw::c_int;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::OnceLock;
use typst::comemo::Tracked;
use typst::diag::{bail, At, FileResult, SourceResult, StrResult};
use typst::engine::Engine;
use typst::foundations::{
    Array, Bytes, Context, Datetime, Dict, IntoValue, NoneValue, Repr, Value,
};
use typst::model::{Numbering, NumberingPattern};
use typst::syntax::{FileId, Source, Span};
use typst::text::{Font, FontBook};
use typst::utils::{tick, LazyHash, SmallBitSet};
use typst::visualize::Color;
use typst::{Features, Library, World};
use typst_kit::fonts::{FontSlot, Fonts};
use typst_library::diag::Warned;
use typst_library::html::HtmlDocument;
use typst_macros::func;

pub type MainCallback = extern "C" fn() -> JavaResult<ExtendedFileDescriptor>;
pub type FileCallback =
    extern "C" fn(ThickBytePtr) -> JavaResult<ExtendedFileResult<Base16ByteArray>>;

/// JavaWorld keeps anything that is needed to impl World from java code with JNA.
/// It is not directly representable with JNA, therefore no #[repr(C)],
/// and JavaWorld is stored and accessed by Pointer
pub struct JavaWorld {
    /// Typst's standard library.
    library: LazyHash<Library>,
    /// Metadata about discovered fonts. TODO make java-compatible
    book: LazyHash<FontBook>,
    /// Callback for World::book method.
    /// Returns c-style string representing a path to a main file.
    main_callback: MainCallback,
    /// Callback for World::file method.
    /// Accepts package: Option<PackageSpec> and path: VirtualPath
    /// Return FileResult<Bytes>
    file_callback: FileCallback,
    /// Fonts, handled as in SystemWorld. TODO make java-compatible
    fonts: Vec<FontSlot>,
    /// File cache
    files: Mutex<HashMap<FileId, FileCache>>,
    /// Now, handled as in SystemWorld
    now: Option<Now>,
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
) -> JavaExceptPtrResult<JavaWorld> {
    tick!();
    let library = unsafe { Box::from_raw(library) }.deref().clone();
    tick!();

    let fonts = Fonts::searcher()
        .include_system_fonts(true)
        .search_with(&(vec![] as Vec<PathBuf>));
    tick!();

    let java_world = JavaWorld {
        library: LazyHash::new(library),
        book: LazyHash::new(fonts.book),
        main_callback,
        file_callback,
        fonts: fonts.fonts,
        files: Mutex::new(HashMap::new()),
        now: now.unpack().into(),
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
                    tick!();
                    let descriptor: ThickBytePtr =
                        serde_json::to_string(&ExtendedFileDescriptor::from(id))
                            .unwrap()
                            .into();
                    tick!();
                    let result = (self.file_callback)(descriptor)
                        .unpack()
                        .map(|it| it.into())
                        .map_err(|it| it.into());
                    tick!();
                    descriptor.release();
                    result
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
                    let descriptor: ThickBytePtr =
                        serde_json::to_string(&ExtendedFileDescriptor::from(id))
                            .unwrap()
                            .into();
                    tick!();
                    let jr = (self.file_callback)(descriptor);
                    tick!("{:?}", jr);
                    let result = jr.unpack().map(|it| it.into()).map_err(|it| it.into());
                    tick!();
                    descriptor.release();
                    tick!();
                    result
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

#[no_mangle]
pub extern "C" fn create_stdlib(features: c_int) -> *mut Library {
    tick!("{:?}", features);
    let inputs: Dict = Dict::new();
    tick!();

    let mut features_bitset = SmallBitSet::default();
    tick!();
    for i in 0..1 {
        if features >> i & 1 == 1 {
            features_bitset.insert(i as usize)
        }
    }
    tick!("{:?}", features_bitset);
    tick!("{:?}", Features(features_bitset.clone()));

    let mut lib = Library::builder()
        .with_inputs(inputs)
        .with_features(Features(features_bitset))
        .build();

    // Temporary, for testing purposes.

    lib.global.scope_mut().define_func::<test>();
    lib.global.scope_mut().define_func::<test_repr>();
    lib.global.scope_mut().define_func::<print>();
    lib.global.scope_mut().define_func::<lines>();
    lib.global
        .scope_mut()
        .define("conifer", Color::from_u8(0x9f, 0xEB, 0x52, 0xFF));
    lib.global
        .scope_mut()
        .define("forest", Color::from_u8(0x43, 0xA1, 0x27, 0xFF));

    tick!();

    Box::into_raw(Box::new(lib))
}

#[func]
fn test(lhs: Value, rhs: Value) -> StrResult<NoneValue> {
    if lhs != rhs {
        bail!("Assertion failed: {} != {}", lhs.repr(), rhs.repr());
    }
    Ok(NoneValue)
}

#[func]
fn test_repr(lhs: Value, rhs: Value) -> StrResult<NoneValue> {
    if lhs.repr() != rhs.repr() {
        bail!("Assertion failed: {} != {}", lhs.repr(), rhs.repr());
    }
    Ok(NoneValue)
}

#[func]
fn print(#[variadic] values: Vec<Value>) -> NoneValue {
    let mut out = std::io::stdout().lock();
    write!(out, "> ").unwrap();
    for (i, value) in values.into_iter().enumerate() {
        if i > 0 {
            write!(out, ", ").unwrap();
        }
        write!(out, "{value:?}").unwrap();
    }
    writeln!(out).unwrap();
    NoneValue
}

/// Generates `count` lines of text based on the numbering.
#[func]
fn lines(
    engine: &mut Engine,
    context: Tracked<Context>,
    span: Span,
    count: usize,
    #[default(Numbering::Pattern(NumberingPattern::from_str("A").unwrap()))]
    numbering: Numbering,
) -> SourceResult<Value> {
    (1..=count)
        .map(|n| numbering.apply(engine, context, &[n]))
        .collect::<SourceResult<Array>>()?
        .join(Some('\n'.into_value()), None)
        .at(span)
}
