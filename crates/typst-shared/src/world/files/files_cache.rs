// Added by LDemetrios

use std::fmt::Debug;
use crate::world::files::cache_cell::CacheCell;
use rustc_hash::FxHashMap;
use std::marker::PhantomData;
use typst::syntax::{FileId, Source};
use typst_library::diag::FileResult;
use typst_library::foundations::Bytes;
use parking_lot::Mutex;

pub trait ReadCallback {
    fn read(&self, id: FileId) -> FileResult<Vec<u8>>;
}

pub struct FilesCache<Reader: ReadCallback + Clone + Copy + Debug> {
    pub  slots: SlotCell<FxHashMap<FileId, FileSlot<Reader>>>,
    pub  reader: Reader,
}

impl<Reader: ReadCallback + Clone + Copy + Debug> FilesCache<Reader> {
    pub fn new(reader: Reader) -> Self {
        Self { slots: SlotCell::new(FxHashMap::default()), reader }
    }

    pub fn reset(&mut self, file_id: FileId) {

        self.slots.with_mut(|map| {
            if let Some(slot) = map.get_mut(&file_id) {
                slot.reset();
            }
        });
    }

    pub fn source(&self, id: FileId) -> FileResult<Source> {

        self.slot(id, |slot| slot.source(&self.reader))
    }

    pub fn file(&self, id: FileId) -> FileResult<Bytes> {

        self.slot(id, |slot| slot.file(&self.reader))
    }

    fn slot<F, T>(&self, id: FileId, f: F) -> T
    where
        F: FnOnce(&mut FileSlot<Reader>) -> T,
    {
        //
        // self.slots.with_mut(|map| {
        //
        //     f(map.entry(id).or_insert_with(|| FileSlot::new(id)))
        // })
        f(&mut FileSlot::new(id))
    }
}

struct SlotCell<T> {
    inner: Mutex<T>,
}

impl<T> SlotCell<T> {
    fn new(value: T) -> Self {
        Self {
            inner: Mutex::new(value),
        }
    }

    fn with_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.inner.lock();
        f(&mut guard)
    }
}

// SAFETY: wasm32 builds are single-threaded for this project, so we can
// treat the RefCell-based SlotCell as Send/Sync there.
#[cfg(target_arch = "wasm32")]
unsafe impl<T: Send> Send for SlotCell<T> {}
#[cfg(target_arch = "wasm32")]
unsafe impl<T: Send> Sync for SlotCell<T> {}

/// Holds the processed data for a file ID.
///
/// Both fields can be populated if the file is both imported and read().
struct FileSlot<Reader: ReadCallback> {
    /// The slot's file id.
    id: FileId,
    /// The lazily loaded and incrementally updated source file.
    source: CacheCell<Source>,
    /// The lazily loaded raw byte buffer.
    file: CacheCell<Bytes>,
    reader: PhantomData<Reader>,
}

impl<Reader: ReadCallback> FileSlot<Reader> {
    /// Create a new file slot.
    fn new(id: FileId) -> Self {
        Self {
            id,
            file: CacheCell::new(),
            source: CacheCell::new(),
            reader: PhantomData::default(),
        }
    }

    /// Whether the file was accessed in the ongoing compilation.
    fn accessed(&self) -> bool {
        self.source.accessed() || self.file.accessed()
    }

    /// Marks the file as not yet accessed in preparation of the next
    /// compilation.
    fn reset(&mut self) {
        self.source.reset();
        self.file.reset();
    }

    /// Retrieve the source for this file.
    fn source(&mut self, reader: &Reader) -> FileResult<Source> {
        self.source.get_or_init(
            || reader.read(self.id),
            |data, prev| {
                let text = decode_utf8(&data)?;
                if let Some(mut prev) = prev {
                    prev.replace(text);
                    Ok(prev)
                } else {
                    Ok(Source::new(self.id, text.into()))
                }
            },
        )
    }

    /// Retrieve the file's bytes.
    fn file(&mut self, reader: &Reader) -> FileResult<Bytes> {
        self.file.get_or_init(|| reader.read(self.id), |data, _| Ok(Bytes::new(data)))
    }
}

/// Decode UTF-8 with an optional BOM.
fn decode_utf8(buf: &[u8]) -> FileResult<&str> {
    // Remove UTF-8 BOM.
    Ok(std::str::from_utf8(buf.strip_prefix(b"\xef\xbb\xbf").unwrap_or(buf))?)
}
